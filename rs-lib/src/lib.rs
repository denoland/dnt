// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use analyze::get_top_level_decls;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;

use analyze::get_ignore_line_indexes;
use anyhow::bail;
use deno_graph::ModuleKind;
use graph::ModuleGraphOptions;
use mappings::Mappings;
use mappings::SYNTHETIC_SPECIFIERS;
use mappings::SYNTHETIC_TEST_SPECIFIERS;
use polyfills::build_polyfill_file;
use polyfills::polyfills_for_target;
use polyfills::Polyfill;
use specifiers::Specifiers;
use text_changes::apply_text_changes;
use text_changes::TextChange;
use utils::get_relative_specifier;
use utils::prepend_statement_to_text;
use visitors::fill_polyfills;
use visitors::get_deno_comment_directive_text_changes;
use visitors::get_global_text_changes;
use visitors::get_import_exports_text_changes;
use visitors::FillPolyfillsParams;
use visitors::GetGlobalTextChangesParams;
use visitors::GetImportExportsTextChangesParams;

pub use deno_ast::ModuleSpecifier;
pub use loader::LoadResponse;
pub use loader::Loader;
pub use utils::url_to_file_path;

use crate::declaration_file_resolution::TypesDependency;
use crate::utils::strip_bom;

mod analyze;
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
#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, PartialEq)]
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
  pub dependencies: Vec<Dependency>,
}

#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(
  feature = "serialization",
  serde(tag = "kind", content = "value", rename_all = "camelCase")
)]
#[derive(Clone, Debug)]
pub enum MappedSpecifier {
  Package(PackageMappedSpecifier),
  Module(ModuleSpecifier),
}

#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug)]
pub struct PackageMappedSpecifier {
  /// Name being mapped to.
  pub name: String,
  /// Version of the specifier. Leave this blank to not have a
  /// dependency (ex. Node modules like "fs")
  pub version: Option<String>,
  /// Sub path of the npm package to use in the module specifier.
  pub sub_path: Option<String>,
}

impl PackageMappedSpecifier {
  pub(crate) fn module_specifier_text(&self) -> String {
    if let Some(path) = &self.sub_path {
      format!("{}/{}", self.name, path)
    } else {
      self.name.clone()
    }
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug)]
pub struct GlobalName {
  /// Name to use as the global name.
  pub name: String,
  /// Optional name of the export from the package.
  pub export_name: Option<String>,
  /// Whether this is a name that only exists as a type declaration.
  pub type_only: bool,
}

#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(
  feature = "serialization",
  serde(tag = "kind", content = "value", rename_all = "camelCase")
)]
#[derive(Clone, Debug)]
pub enum Shim {
  Package(PackageShim),
  Module(ModuleShim),
}

impl Shim {
  pub fn global_names(&self) -> &Vec<GlobalName> {
    match self {
      Shim::Package(shim) => &shim.global_names,
      Shim::Module(shim) => &shim.global_names,
    }
  }

  pub(crate) fn maybe_specifier(&self) -> Option<ModuleSpecifier> {
    match self {
      Shim::Package(_) => None,
      Shim::Module(module) => module.maybe_specifier(),
    }
  }
}

#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug)]
pub struct PackageShim {
  /// Information about the npm package to use for this shim.
  pub package: PackageMappedSpecifier,
  /// Npm package to include in the dev depedencies that has the type declarations.
  pub types_package: Option<Dependency>,
  /// Names this shim provides that will be injected in global contexts.
  pub global_names: Vec<GlobalName>,
}

#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug)]
pub struct ModuleShim {
  /// Information about the module or bare specifier to use for this shim.
  pub module: String,
  /// Names this shim provides that will be injected in global contexts.
  pub global_names: Vec<GlobalName>,
}

impl ModuleShim {
  pub fn maybe_specifier(&self) -> Option<ModuleSpecifier> {
    if self.module.starts_with("node:") {
      None
    } else {
      ModuleSpecifier::parse(&self.module).ok()
    }
  }
}

// make sure to update `ScriptTarget` in the TS code when changing the names on this
#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[derive(Clone, Copy, Debug)]
pub enum ScriptTarget {
  ES3 = 0,
  ES5 = 1,
  ES2015 = 2,
  ES2016 = 3,
  ES2017 = 4,
  ES2018 = 5,
  ES2019 = 6,
  ES2020 = 7,
  ES2021 = 8,
  Latest = 9,
}

pub struct TransformOptions {
  pub entry_points: Vec<ModuleSpecifier>,
  pub test_entry_points: Vec<ModuleSpecifier>,
  pub shims: Vec<Shim>,
  pub test_shims: Vec<Shim>,
  pub loader: Option<Box<dyn Loader>>,
  /// Maps specifiers to an npm package or module.
  pub specifier_mappings: HashMap<ModuleSpecifier, MappedSpecifier>,
  /// Version of ECMAScript that the final code will target.
  /// This controls whether certain polyfills should occur.
  pub target: ScriptTarget,
  /// Optional import map.
  pub import_map: Option<ModuleSpecifier>,
}

struct EnvironmentContext<'a> {
  environment: TransformOutputEnvironment,
  searching_polyfills: Vec<Box<dyn Polyfill>>,
  found_polyfills: Vec<Box<dyn Polyfill>>,
  shim_file_specifier: &'a ModuleSpecifier,
  shim_global_names: HashSet<&'a str>,
  shims: &'a Vec<Shim>,
  used_shim: bool,
}

pub async fn transform(options: TransformOptions) -> Result<TransformOutput> {
  if options.entry_points.is_empty() {
    anyhow::bail!("at least one entry point must be specified");
  }

  let (module_graph, specifiers) =
    crate::graph::ModuleGraph::build_with_specifiers(ModuleGraphOptions {
      entry_points: options
        .entry_points
        .iter()
        .cloned()
        .chain(options.shims.iter().filter_map(|s| s.maybe_specifier()))
        .collect(),
      test_entry_points: options
        .test_entry_points
        .iter()
        .cloned()
        .chain(
          options
            .test_shims
            .iter()
            .filter_map(|s| s.maybe_specifier()),
        )
        .collect(),
      specifier_mappings: &options.specifier_mappings,
      loader: options.loader,
      import_map: options.import_map,
    })
    .await?;

  let mappings = Mappings::new(&module_graph, &specifiers)?;
  let all_package_specifier_mappings: HashMap<ModuleSpecifier, String> =
    specifiers
      .main
      .mapped
      .iter()
      .chain(specifiers.test.mapped.iter())
      .map(|m| (m.0.clone(), m.1.module_specifier_text()))
      .collect();

  let mut warnings = get_declaration_warnings(&specifiers);
  let mut main_env_context = EnvironmentContext {
    environment: TransformOutputEnvironment {
      entry_points: options
        .entry_points
        .iter()
        .map(|p| mappings.get_file_path(p).to_owned())
        .collect(),
      dependencies: get_dependencies(specifiers.main.mapped),
      ..Default::default()
    },
    searching_polyfills: polyfills_for_target(options.target),
    found_polyfills: Default::default(),
    shim_file_specifier: &SYNTHETIC_SPECIFIERS.shims,
    shim_global_names: options
      .shims
      .iter()
      .flat_map(|s| s.global_names().iter().map(|s| s.name.as_str()))
      .collect(),
    shims: &options.shims,
    used_shim: false,
  };
  let mut test_env_context = EnvironmentContext {
    environment: TransformOutputEnvironment {
      entry_points: options
        .test_entry_points
        .iter()
        .map(|p| mappings.get_file_path(p).to_owned())
        .collect(),
      dependencies: get_dependencies(specifiers.test.mapped),
      ..Default::default()
    },
    searching_polyfills: polyfills_for_target(options.target),
    found_polyfills: Default::default(),
    shim_file_specifier: &SYNTHETIC_TEST_SPECIFIERS.shims,
    shim_global_names: options
      .test_shims
      .iter()
      .flat_map(|s| s.global_names().iter().map(|s| s.name.as_str()))
      .collect(),
    shims: &options.test_shims,
    used_shim: false,
  };

  for specifier in specifiers
    .local
    .iter()
    .chain(specifiers.remote.iter())
    .chain(specifiers.types.iter().map(|(_, d)| &d.selected.specifier))
  {
    let module = module_graph.get(specifier);
    let env_context = if specifiers.test_modules.contains(specifier) {
      &mut test_env_context
    } else {
      &mut main_env_context
    };

    let file_text = match module.kind {
      ModuleKind::Esm => {
        let parsed_source = module
          .maybe_parsed_source
          .as_ref()
          .ok_or_else(|| anyhow!("Expected source for: {}", module.specifier))?
          .clone();

        let text_changes = parsed_source
          .with_view(|program| -> Result<Vec<TextChange>> {
            let top_level_context = parsed_source.top_level_context();
            let ignore_line_indexes =
              get_ignore_line_indexes(parsed_source.specifier(), &program);
            let top_level_decls =
              get_top_level_decls(&program, top_level_context);
            warnings.extend(ignore_line_indexes.warnings);

            fill_polyfills(&mut FillPolyfillsParams {
              found_polyfills: &mut env_context.found_polyfills,
              searching_polyfills: &mut env_context.searching_polyfills,
              program: &program,
              top_level_context: parsed_source.top_level_context(),
              top_level_decls: &top_level_decls,
            });

            let mut text_changes = Vec::new();

            // shim changes
            {
              let shim_relative_specifier = get_relative_specifier(
                mappings.get_file_path(specifier),
                mappings.get_file_path(env_context.shim_file_specifier),
              );
              let result =
                get_global_text_changes(&GetGlobalTextChangesParams {
                  program: &program,
                  top_level_context,
                  shim_specifier: &shim_relative_specifier,
                  shim_global_names: &env_context.shim_global_names,
                  ignore_line_indexes: &ignore_line_indexes.line_indexes,
                  top_level_decls: &top_level_decls,
                });
              text_changes.extend(result.text_changes);
              if result.imported_shim {
                env_context.used_shim = true;
              }
            }

            text_changes
              .extend(get_deno_comment_directive_text_changes(&program));
            text_changes.extend(get_import_exports_text_changes(
              &GetImportExportsTextChangesParams {
                specifier,
                module_graph: &module_graph,
                mappings: &mappings,
                program: &program,
                package_specifier_mappings: &all_package_specifier_mappings,
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
      ModuleKind::Asserted => {
        if let Some(source) = &module.maybe_source {
          format!(
            "export default JSON.parse(`{}`);",
            strip_bom(&source.replace('`', "\\`").replace("${", "\\${"))
          )
        } else {
          continue;
        }
      }
      _ => bail!(
        "Not implemented module kind {:?} for {}",
        module.kind,
        module.specifier
      ),
    };

    let file_path = mappings.get_file_path(specifier).to_owned();
    env_context.environment.files.push(OutputFile {
      file_path,
      file_text,
    });
  }

  check_add_polyfill_file_to_environment(
    &mut main_env_context,
    mappings.get_file_path(&SYNTHETIC_SPECIFIERS.polyfills),
  );
  check_add_polyfill_file_to_environment(
    &mut test_env_context,
    mappings.get_file_path(&SYNTHETIC_TEST_SPECIFIERS.polyfills),
  );
  check_add_shim_file_to_environment(
    &mut main_env_context,
    mappings.get_file_path(&SYNTHETIC_SPECIFIERS.shims),
    &mappings,
  );
  check_add_shim_file_to_environment(
    &mut test_env_context,
    mappings.get_file_path(&SYNTHETIC_TEST_SPECIFIERS.shims),
    &mappings,
  );

  add_shim_types_packages_to_test_environment(
    &mut test_env_context.environment,
    options.shims.iter().chain(options.test_shims.iter()),
  );

  // Remove any dependencies from the test environment that
  // are found in the main environment. Only check for exact
  // matches in order to cause an npm install error if there
  // are two dependencies with the same name, but different versions.
  test_env_context.environment.dependencies = test_env_context
    .environment
    .dependencies
    .into_iter()
    .filter(|d| !main_env_context.environment.dependencies.contains(d))
    .collect();

  Ok(TransformOutput {
    main: main_env_context.environment,
    test: test_env_context.environment,
    warnings,
  })
}

fn add_shim_types_packages_to_test_environment<'a>(
  test_output_env: &mut TransformOutputEnvironment,
  all_shims: impl Iterator<Item = &'a Shim>,
) {
  for shim in all_shims {
    if let Shim::Package(shim) = shim {
      if let Some(types_package) = &shim.types_package {
        test_output_env.dependencies.push(types_package.clone())
      }
    }
  }
}

fn check_add_polyfill_file_to_environment(
  env_context: &mut EnvironmentContext,
  polyfill_file_path: &Path,
) {
  if let Some(polyfill_file_text) =
    build_polyfill_file(&env_context.found_polyfills)
  {
    env_context.environment.files.push(OutputFile {
      file_path: polyfill_file_path.to_path_buf(),
      file_text: polyfill_file_text,
    });

    for entry_point in env_context.environment.entry_points.iter() {
      if let Some(file) = env_context
        .environment
        .files
        .iter_mut()
        .find(|f| &f.file_path == entry_point)
      {
        prepend_statement_to_text(
          &file.file_path,
          &mut file.file_text,
          &format!(
            "import \"{}\";",
            get_relative_specifier(&file.file_path, &polyfill_file_path)
          ),
        );
      }
    }
  }
}

fn check_add_shim_file_to_environment(
  env_context: &mut EnvironmentContext,
  shim_file_path: &Path,
  mappings: &Mappings,
) {
  if env_context.used_shim {
    let shim_file_text =
      build_shim_file(env_context.shims, shim_file_path, mappings);
    env_context.environment.files.push(OutputFile {
      file_path: shim_file_path.to_path_buf(),
      file_text: shim_file_text,
    });

    for shim in env_context.shims.iter() {
      if let Shim::Package(shim) = shim {
        if !env_context
          .environment
          .dependencies
          .iter()
          .any(|d| d.name == shim.package.name)
        {
          if let Some(version) = &shim.package.version {
            env_context.environment.dependencies.push(Dependency {
              name: shim.package.name.to_string(),
              version: version.clone(),
            });
          }
        }
      }
    }
  }

  fn build_shim_file(
    shims: &[Shim],
    shim_file_path: &Path,
    mappings: &Mappings,
  ) -> String {
    fn get_specifer_text(n: &GlobalName) -> String {
      let name_text = if let Some(export_name) = &n.export_name {
        format!("{} as {}", export_name, n.name)
      } else {
        n.name.to_string()
      };
      if n.type_only {
        format!("type {}", name_text)
      } else {
        name_text
      }
    }

    fn get_module_specifier_text(
      shim: &Shim,
      shim_file_path: &Path,
      mappings: &Mappings,
    ) -> String {
      match shim {
        Shim::Package(shim) => shim.package.module_specifier_text(),
        Shim::Module(shim) => match shim.maybe_specifier() {
          Some(specifier) => {
            let to = mappings.get_file_path(&specifier);
            get_relative_specifier(shim_file_path, to)
          }
          None => shim.module.clone(),
        },
      }
    }

    let mut text = String::new();
    for shim in shims.iter() {
      let declaration_names = shim
        .global_names()
        .iter()
        .filter(|n| !n.type_only)
        .collect::<Vec<_>>();
      let module_specifier_text =
        get_module_specifier_text(shim, shim_file_path, mappings);
      if !declaration_names.is_empty() {
        text.push_str(&format!(
          "import {{ {} }} from \"{}\";\n",
          declaration_names
            .into_iter()
            .map(get_specifer_text)
            .collect::<Vec<_>>()
            .join(", "),
          &module_specifier_text,
        ));
      }

      text.push_str(&format!(
        "export {{ {} }} from \"{}\";\n",
        shim
          .global_names()
          .iter()
          .map(get_specifer_text)
          .collect::<Vec<_>>()
          .join(", "),
        &module_specifier_text,
      ));
    }

    if !text.is_empty() {
      text.push('\n');
    }

    text.push_str("const dntGlobals = {\n");
    for global_name in shims.iter().flat_map(|s| s.global_names().iter()) {
      if !global_name.type_only {
        text.push_str(&format!("  {},\n", global_name.name));
      }
    }
    text.push_str("};\n");
    text.push_str("export const dntGlobalThis = createMergeProxy(globalThis, dntGlobals);\n\n");

    text.push_str(
      &include_str!("scripts/createMergeProxy.ts")
        .replace("export function", "function"),
    );

    text
  }
}

fn get_dependencies(
  mappings: BTreeMap<ModuleSpecifier, PackageMappedSpecifier>,
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
  dependencies.dedup(); // only works after sorting
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
