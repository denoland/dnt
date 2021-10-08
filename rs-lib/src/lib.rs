// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use deno_graph::create_graph;
use deno_graph::ModuleGraph;
use deno_graph::Resolved;
#[macro_use]
extern crate lazy_static;

use loader::SourceLoader;
use loader::get_all_specifier_mappers;
use mappings::Mappings;
use mappings::Specifiers;
use text_changes::apply_text_changes;
use visitors::get_deno_global_text_changes;
use visitors::get_module_specifier_text_changes;
use visitors::GetDenoGlobalTextChangesParams;
use visitors::GetModuleSpecifierTextChangesParams;

pub use deno_ast::ModuleSpecifier;
pub use loader::LoadResponse;
pub use loader::Loader;

use crate::loader::MappedSpecifierEntry;

mod loader;
mod mappings;
mod parser;
mod text_changes;
mod utils;
mod visitors;

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq)]
pub struct OutputFile {
  pub file_path: PathBuf,
  pub file_text: String,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq)]
pub struct Dependency {
  pub name: String,
  pub version: String,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq)]
pub struct TransformOutput {
  pub entry_point_file_path: String,
  pub shim_used: bool,
  pub dependencies: Vec<Dependency>,
  pub files: Vec<OutputFile>,
}

pub struct TransformOptions {
  pub entry_point: ModuleSpecifier,
  pub shim_package_name: String,
  pub loader: Option<Box<dyn Loader>>,
  pub specifier_mappings: Option<HashMap<ModuleSpecifier, String>>,
}

pub async fn transform(options: TransformOptions) -> Result<TransformOutput> {
  let shim_package_name = options.shim_package_name;
  let ignored_specifiers = options.specifier_mappings.as_ref().map(|t| t.keys().map(ToOwned::to_owned).collect());
  let mut loader =
    loader::SourceLoader::new(options.loader.unwrap_or_else(|| {
      #[cfg(feature = "tokio-loader")]
      return Box::new(loader::DefaultLoader::new());
      #[cfg(not(feature = "tokio-loader"))]
      panic!("You must provide a loader or use the 'tokio-loader' feature.")
    }),
    // todo: support configuring this in the future
    get_all_specifier_mappers(),
    ignored_specifiers.as_ref(),
  );
  let source_parser = parser::CapturingSourceParser::new();
  let module_graph = create_graph(
    options.entry_point.clone(),
    &mut loader,
    None,
    None,
    Some(&source_parser),
  )
  .await;

  let specifiers = get_specifiers_from_loader(loader, &module_graph)?;

  if let Some(ignored_specifiers) = ignored_specifiers {
    let mut not_found_specifiers = ignored_specifiers.into_iter().filter(|s| !specifiers.found_ignored.contains(s)).collect::<Vec<_>>();
    if !not_found_specifiers.is_empty() {
      not_found_specifiers.sort();
      anyhow::bail!(
        "The following specifiers were indicated to be mapped, but were not found:\n{}",
        not_found_specifiers.into_iter().map(|s| format!("  * {}", s)).collect::<Vec<_>>().join("\n"),
      );
    }
  }

  let mappings = Mappings::new(&module_graph, &specifiers)?;
  let mut specifier_mappings = options.specifier_mappings;
  if !specifiers.mapped.is_empty() {
    if specifier_mappings.is_none() {
      specifier_mappings = Some(HashMap::new());
    }
    let specifier_mappings = specifier_mappings.as_mut().unwrap();
    for entry in specifiers.mapped.iter() {
      specifier_mappings.insert(entry.from_specifier.clone(), entry.to_specifier.clone());
    }
  }

  // todo: parallelize
  let mut files = Vec::new();
  let mut shim_used = false;
  for specifier in specifiers
    .local
    .iter()
    .chain(specifiers.remote.iter())
    .chain(specifiers.types.iter().map(|(_, from)| from))
  {
    let parsed_source = source_parser.get_parsed_source(specifier)?;

    let text_changes = parsed_source.with_view(|program| {
      let mut text_changes =
        get_deno_global_text_changes(&GetDenoGlobalTextChangesParams {
          program: &program,
          top_level_context: parsed_source.top_level_context(),
          shim_package_name: shim_package_name.as_str(),
        });
      if !text_changes.is_empty() {
        shim_used = true;
      }
      text_changes.extend(get_module_specifier_text_changes(
        &GetModuleSpecifierTextChangesParams {
          specifier,
          module_graph: &module_graph,
          mappings: &mappings,
          program: &program,
          specifier_mappings: specifier_mappings.as_ref(),
        },
      ));

      text_changes
    });

    let file_path = mappings.get_file_path(specifier).to_owned();
    files.push(OutputFile {
      file_path,
      file_text: apply_text_changes(
        parsed_source.source().text().to_string(),
        text_changes,
      ),
    });
  }

  Ok(TransformOutput {
    entry_point_file_path: mappings
      .get_file_path(&options.entry_point)
      .to_string_lossy()
      .to_string(),
    dependencies: get_dependencies(specifiers),
    shim_used,
    files,
  })
}

fn get_specifiers_from_loader(
  loader: SourceLoader,
  module_graph: &ModuleGraph,
) -> Result<Specifiers> {
  let specifiers = loader.into_specifiers();
  let mut types = BTreeMap::new();

  handle_specifiers(&specifiers.local, module_graph, &mut types)?;
  handle_specifiers(&specifiers.remote, module_graph, &mut types)?;

  let type_specifiers = types.values().collect::<HashSet<_>>();

  return Ok(Specifiers {
    local: specifiers
      .local
      .into_iter()
      .filter(|l| !type_specifiers.contains(&l))
      .collect(),
    remote: specifiers
      .remote
      .into_iter()
      .filter(|l| !type_specifiers.contains(&l))
      .collect(),
    types,
    found_ignored: specifiers.found_ignored,
    mapped: get_mapped(specifiers.mapped)?,
  });

  fn handle_specifiers(
    specifiers: &[ModuleSpecifier],
    module_graph: &ModuleGraph,
    types: &mut BTreeMap<ModuleSpecifier, ModuleSpecifier>,
  ) -> Result<()> {
    for specifier in specifiers {
      let module = module_graph.try_get(specifier).map_err(|err| {
        anyhow::anyhow!("{} ({})", err.to_string(), specifier)
      })?;
      let module = module
        .unwrap_or_else(|| panic!("Could not find module for {}", specifier));

      match &module.maybe_types_dependency {
        Some((text, Resolved::Err(err, _))) => anyhow::bail!(
          "Error resolving types for {} with reference {}. {}",
          specifier,
          text,
          err.to_string()
        ),
        Some((_, Resolved::Specifier(type_specifier, _))) => {
          types.insert(specifier.clone(), type_specifier.clone());
        }
        _ => {}
      }
    }

    Ok(())
  }

  fn get_mapped(mapped_specifiers: Vec<MappedSpecifierEntry>) -> Result<Vec<MappedSpecifierEntry>> {
    let mut specifier_for_name: HashMap<String, MappedSpecifierEntry> = HashMap::new();
    let mut result = Vec::new();
    for mapped_specifier in mapped_specifiers {
      if let Some(specifier) = specifier_for_name.get(&mapped_specifier.to_specifier) {
        if specifier.version != mapped_specifier.version {
          anyhow::bail!("Specifier {} with version {} did not match specifier {} with version {}.",
            specifier.from_specifier,
            specifier.version.as_ref().map(|v| v.as_str()).unwrap_or("<unknown>"),
            mapped_specifier.from_specifier,
            mapped_specifier.version.as_ref().map(|v| v.as_str()).unwrap_or("<unknown>"),
          );
        }
      } else {
        specifier_for_name.insert(mapped_specifier.to_specifier.to_string(), mapped_specifier.clone());
        result.push(mapped_specifier);
      }
    }

    Ok(result)
  }
}

fn get_dependencies(specifiers: Specifiers) -> Vec<Dependency> {
  specifiers.mapped.into_iter().filter_map(|entry| {
    if let Some(version) = entry.version {
      Some(Dependency {
        name: entry.to_specifier,
        version,
      })
    } else {
      None
    }
  }).collect()
}
