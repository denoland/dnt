// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
#[macro_use]
extern crate lazy_static;

use graph::ModuleGraphOptions;
use mappings::Mappings;
use specifiers::Specifiers;
use text_changes::apply_text_changes;
use visitors::get_deno_comment_directive_text_changes;
use visitors::get_deno_global_text_changes;
use visitors::get_module_specifier_text_changes;
use visitors::GetDenoGlobalTextChangesParams;
use visitors::GetModuleSpecifierTextChangesParams;

pub use deno_ast::ModuleSpecifier;
pub use loader::LoadResponse;
pub use loader::Loader;
pub use utils::url_to_file_path;

use crate::declaration_file_resolution::TypesDependency;

mod declaration_file_resolution;
mod graph;
mod loader;
mod mappings;
mod parser;
mod specifiers;
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
  pub warnings: Vec<String>,
}

pub struct TransformOptions {
  pub entry_point: ModuleSpecifier,
  pub shim_package_name: String,
  pub loader: Option<Box<dyn Loader>>,
  pub specifier_mappings: Option<HashMap<ModuleSpecifier, String>>,
}

pub async fn transform(options: TransformOptions) -> Result<TransformOutput> {
  let shim_package_name = options.shim_package_name;
  let ignored_specifiers = options
    .specifier_mappings
    .as_ref()
    .map(|t| t.keys().map(ToOwned::to_owned).collect());

  let (module_graph, specifiers) =
    crate::graph::ModuleGraph::build_with_specifiers(ModuleGraphOptions {
      entry_point: options.entry_point.clone(),
      ignored_specifiers: ignored_specifiers.as_ref(),
      loader: options.loader,
    })
    .await?;

  let mappings = Mappings::new(&module_graph, &specifiers)?;
  let mut specifier_mappings = options.specifier_mappings;
  if !specifiers.mapped.is_empty() {
    if specifier_mappings.is_none() {
      specifier_mappings = Some(HashMap::new());
    }
    let specifier_mappings = specifier_mappings.as_mut().unwrap();
    for entry in specifiers.mapped.values() {
      specifier_mappings
        .insert(entry.from_specifier.clone(), entry.to_specifier.clone());
    }
  }

  // todo: parallelize
  let mut files = Vec::new();
  let mut shim_used = false;
  for specifier in specifiers
    .local
    .iter()
    .chain(specifiers.remote.iter())
    .chain(specifiers.types.iter().map(|(_, d)| &d.selected.specifier))
  {
    let module = module_graph.get(specifier);
    let parsed_source = module.parsed_source.clone();

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
      text_changes.extend(get_deno_comment_directive_text_changes(&program));
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
    warnings: get_declaration_warnings(&specifiers),
    dependencies: get_dependencies(specifiers),
    shim_used,
    files,
  })
}

fn get_dependencies(specifiers: Specifiers) -> Vec<Dependency> {
  specifiers
    .mapped
    .into_iter()
    .filter_map(|entry| {
      if let Some(version) = entry.1.version {
        Some(Dependency {
          name: entry.1.to_specifier,
          version,
        })
      } else {
        None
      }
    })
    .collect()
}

fn get_declaration_warnings(specifiers: &Specifiers) -> Vec<String> {
  let mut messages = Vec::new();
  for (code_specifier, d) in specifiers.types.iter() {
    if d.selected.referrer.scheme() == "file" {
      let local_referrers =
        d.ignored.iter().filter(|d| d.referrer.scheme() == "file");
      for dep in local_referrers {
        messages.push(get_dep_warning(
          code_specifier,
          dep,
          &d.selected,
          "Supress this warning by having only one local file specify the declaration file for this module.",
        ));
      }
    } else {
      for dep in d.ignored.iter() {
        messages.push(get_dep_warning(
          code_specifier,
          dep,
          &d.selected,
          "Supress this warning by specifying a declaration file for this module locally via `@deno-types`.",
        ));
      }
    }
  }
  return messages;

  fn get_dep_warning(
    code_specifier: &ModuleSpecifier,
    dep: &TypesDependency,
    selected_dep: &TypesDependency,
    post_message: &str,
  ) -> String {
    format!("Duplicate declaration file found for {}\n  Specified {} in {}\n  Selected {}\n  {}", code_specifier, dep.specifier, dep.referrer, selected_dep.specifier, post_message)
  }
}
