// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
#[macro_use]
extern crate lazy_static;

use graph::ModuleGraphOptions;
use graph::ModuleRef;
use mappings::Mappings;
use polyfills::build_polyfill_file;
use polyfills::Polyfill;
use specifiers::Specifiers;
use text_changes::apply_text_changes;
use text_changes::TextChange;
use utils::get_relative_specifier;
use visitors::fill_polyfills;
use visitors::get_deno_comment_directive_text_changes;
use visitors::get_deno_global_text_changes;
use visitors::get_ignore_line_indexes;
use visitors::get_import_exports_text_changes;
use visitors::FillPolyfillsParams;
use visitors::GetDenoGlobalTextChangesParams;
use visitors::GetImportExportsTextChangesParams;

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
mod polyfills;
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
  pub main: TransformOutputEnvironment,
  pub test: TransformOutputEnvironment,
  pub warnings: Vec<String>,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Default)]
pub struct TransformOutputEnvironment {
  pub entry_points: Vec<PathBuf>,
  pub files: Vec<OutputFile>,
  pub shim_used: bool,
  pub dependencies: Vec<Dependency>,
}

#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct MappedSpecifier {
  /// Name being mapped to.
  pub name: String,
  /// Version of the specifier. Leave this blank to not have a
  /// dependency (ex. Node modules like "path")
  pub version: Option<String>,
}

pub struct TransformOptions {
  pub entry_points: Vec<ModuleSpecifier>,
  pub test_entry_points: Vec<ModuleSpecifier>,
  pub shim_package_name: String,
  pub loader: Option<Box<dyn Loader>>,
  /// Maps specifiers to an npm module. This does not follow or resolve
  /// the mapped specifier
  pub specifier_mappings: HashMap<ModuleSpecifier, MappedSpecifier>,
  /// Redirects one specifier to another specifier.
  pub redirects: HashMap<ModuleSpecifier, ModuleSpecifier>,
}

pub async fn transform(options: TransformOptions) -> Result<TransformOutput> {
  if options.entry_points.is_empty() {
    anyhow::bail!("at least one entry point must be specified");
  }

  let shim_package_name = options.shim_package_name;
  let (module_graph, specifiers) =
    crate::graph::ModuleGraph::build_with_specifiers(ModuleGraphOptions {
      entry_points: options.entry_points.clone(),
      test_entry_points: options.test_entry_points.clone(),
      specifier_mappings: &options.specifier_mappings,
      redirects: &options.redirects,
      loader: options.loader,
    })
    .await?;

  let mappings = Mappings::new(&module_graph, &specifiers)?;
  let all_specifier_mappings: HashMap<ModuleSpecifier, String> = specifiers
    .main
    .mapped
    .iter()
    .chain(specifiers.test.mapped.iter())
    .map(|m| (m.0.clone(), m.1.name.clone()))
    .collect();

  // todo: parallelize
  let mut warnings = get_declaration_warnings(&specifiers);
  let mut main_environment = TransformOutputEnvironment {
    entry_points: options
      .entry_points
      .iter()
      .map(|p| mappings.get_file_path(p).to_owned())
      .collect(),
    dependencies: get_dependencies(specifiers.main.mapped),
    ..Default::default()
  };
  let mut test_environment = TransformOutputEnvironment {
    entry_points: options
      .test_entry_points
      .iter()
      .map(|p| mappings.get_file_path(p).to_owned())
      .collect(),
    dependencies: get_dependencies(specifiers.test.mapped),
    ..Default::default()
  };
  let mut main_polyfills = HashSet::new();
  let mut test_polyfills = HashSet::new();

  for specifier in specifiers
    .local
    .iter()
    .chain(specifiers.remote.iter())
    .chain(specifiers.types.iter().map(|(_, d)| &d.selected.specifier))
  {
    let module = module_graph.get(specifier);
    let (environment, polyfills) =
      if specifiers.test_modules.contains(specifier) {
        (&mut test_environment, &mut test_polyfills)
      } else {
        (&mut main_environment, &mut main_polyfills)
      };

    let file_text = match module {
      ModuleRef::Es(module) => {
        let parsed_source = module.parsed_source.clone();

        let text_changes = parsed_source
          .with_view(|program| -> Result<Vec<TextChange>> {
            let ignore_line_indexes =
              get_ignore_line_indexes(parsed_source.specifier(), &program);
            warnings.extend(ignore_line_indexes.warnings);

            fill_polyfills(&mut FillPolyfillsParams {
              polyfills,
              program: &program,
              top_level_context: parsed_source.top_level_context(),
            });

            let mut text_changes = Vec::new();

            // shim changes
            {
              let shim_changes =
                get_deno_global_text_changes(&GetDenoGlobalTextChangesParams {
                  program: &program,
                  top_level_context: parsed_source.top_level_context(),
                  shim_package_name: shim_package_name.as_str(),
                  ignore_line_indexes: &ignore_line_indexes.line_indexes,
                });
              if !shim_changes.is_empty() {
                environment.shim_used = true;
              }
              text_changes.extend(shim_changes);
            }

            text_changes
              .extend(get_deno_comment_directive_text_changes(&program));
            text_changes.extend(get_import_exports_text_changes(
              &GetImportExportsTextChangesParams {
                specifier,
                module_graph: &module_graph,
                mappings: &mappings,
                program: &program,
                specifier_mappings: &all_specifier_mappings,
              },
            )?);

            Ok(text_changes)
          })
          .with_context(|| {
            format!(
              "Issue getting text changes from {}",
              parsed_source.specifier()
            )
          })?;

        apply_text_changes(
          parsed_source.source().text().to_string(),
          text_changes,
        )
      }
      ModuleRef::Synthetic(_) => {
        continue; // these are inlined
      }
    };

    let file_path = mappings.get_file_path(specifier).to_owned();
    environment.files.push(OutputFile {
      file_path,
      file_text,
    });
  }

  // assumes that these file names won't be in the regular output
  check_add_polyfills_to_environment(
    &main_polyfills,
    &mut main_environment,
    "_dnt.polyfills.ts",
  );
  check_add_polyfills_to_environment(
    &test_polyfills,
    &mut test_environment,
    "_dnt.test_polyfills.ts",
  );

  Ok(TransformOutput {
    main: main_environment,
    test: test_environment,
    warnings,
  })
}

fn check_add_polyfills_to_environment(
  polyfills: &HashSet<Polyfill>,
  environment: &mut TransformOutputEnvironment,
  polyfill_path: impl AsRef<Path>,
) {
  if let Some(polyfill_file_text) = build_polyfill_file(polyfills) {
    environment.files.push(OutputFile {
      file_path: polyfill_path.as_ref().to_path_buf(),
      file_text: polyfill_file_text,
    });

    for entry_point in environment.entry_points.iter() {
      if let Some(file) = environment
        .files
        .iter_mut()
        .find(|f| &f.file_path == entry_point)
      {
        file.file_text = format!(
          "import '{}';\n{}",
          get_relative_specifier(&file.file_path, &polyfill_path),
          file.file_text
        );
      }
    }
  }
}

fn get_dependencies(
  mappings: BTreeMap<ModuleSpecifier, MappedSpecifier>,
) -> Vec<Dependency> {
  let mut dependencies = mappings
    .into_iter()
    .filter_map(|entry| {
      if let Some(version) = entry.1.version {
        Some(Dependency {
          name: entry.1.name,
          version,
        })
      } else {
        None
      }
    })
    .collect::<Vec<_>>();
  dependencies.sort_by(|a, b| a.name.cmp(&b.name));
  dependencies
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
