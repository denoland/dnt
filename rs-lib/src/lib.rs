// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use deno_graph::create_graph;
use deno_graph::ModuleGraph;
use deno_graph::Resolved;
#[macro_use]
extern crate lazy_static;

use loader::SourceLoader;
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
pub struct TransformOutput {
  pub entry_point_file_path: String,
  pub cjs_files: Vec<OutputFile>,
  pub mjs_files: Vec<OutputFile>,
}

pub struct TransformOptions {
  pub entry_point: ModuleSpecifier,
  pub shim_package_name: Option<String>,
  pub loader: Option<Box<dyn Loader>>,
}

pub async fn transform(options: TransformOptions) -> Result<TransformOutput> {
  let shim_package_name = options
    .shim_package_name
    .unwrap_or_else(|| "shim-package-name".to_string());
  let mut loader =
    loader::SourceLoader::new(options.loader.unwrap_or_else(|| {
      #[cfg(feature = "tokio-loader")]
      return Box::new(loader::DefaultLoader::new());
      #[cfg(not(feature = "tokio-loader"))]
      panic!("You must provide a loader or use the 'tokio-loader' feature.")
    }));
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

  let mappings = Mappings::new(&module_graph, &specifiers)?;

  // todo: parallelize
  let mut cjs_files = Vec::new();
  let mut mjs_files = Vec::new();
  for specifier in specifiers
    .local
    .iter()
    .chain(specifiers.remote.iter())
    .chain(specifiers.types.iter().map(|(_, from)| from))
  {
    let parsed_source = source_parser.get_parsed_source(specifier)?;

    let (cjs_changes, mjs_changes) = parsed_source.with_view(|program| {
      let common_changes = get_deno_global_text_changes(
        &GetDenoGlobalTextChangesParams {
          program: &program,
          top_level_context: parsed_source.top_level_context(),
          shim_package_name: shim_package_name.as_str(),
        },
      );
      let mut cjs_changes = get_module_specifier_text_changes(
        &GetModuleSpecifierTextChangesParams {
          specifier,
          module_graph: &module_graph,
          mappings: &mappings,
          use_js_extension: false,
          program: &program,
        },
      );
      cjs_changes.extend(common_changes.clone());
      let mut mjs_changes = get_module_specifier_text_changes(
        &GetModuleSpecifierTextChangesParams {
          specifier,
          module_graph: &module_graph,
          mappings: &mappings,
          use_js_extension: true,
          program: &program,
        },
      );
      mjs_changes.extend(common_changes);

      (cjs_changes, mjs_changes)
    });

    let file_path = mappings.get_file_path(specifier).to_owned();
    cjs_files.push(OutputFile {
      file_path: file_path.clone(),
      file_text: apply_text_changes(
        parsed_source.source().text().to_string(),
        cjs_changes,
      ),
    });
    mjs_files.push(OutputFile {
      file_path,
      file_text: apply_text_changes(
        parsed_source.source().text().to_string(),
        mjs_changes,
      ),
    });
  }

  Ok(TransformOutput {
    entry_point_file_path: mappings.get_file_path(&options.entry_point).to_string_lossy().to_string(),
    cjs_files,
    mjs_files,
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
}
