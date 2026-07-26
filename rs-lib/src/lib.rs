// Copyright 2018-2024 the Deno authors. MIT license.

#![allow(clippy::bool_assert_comparison)]
#![deny(clippy::disallowed_methods)]
#![deny(clippy::disallowed_types)]

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use analyze::get_top_level_decls;
use anyhow::Context;
use anyhow::Result;

use crate::loader::get_all_specifier_mappers;
use crate::loader::get_types_package_for_specifier;
use analyze::get_ignore_line_indexes;
use anyhow::bail;
use deno_ast::apply_text_changes;
use deno_ast::TextChange;
use deno_cache_dir::file_fetcher::NullBlobStore;
use deno_graph::Module;
use deno_resolver::deno_json::CompilerOptionsOverrides;
use deno_resolver::factory::ConfigDiscoveryOption;
use deno_resolver::factory::ResolverFactoryOptions;
use deno_resolver::factory::SpecifiedImportMapProvider;
use deno_resolver::factory::WorkspaceFactoryOptions;
use deno_resolver::factory::WorkspaceFactorySys;
use deno_resolver::file_fetcher::DenoGraphLoader;
use deno_resolver::file_fetcher::DenoGraphLoaderOptions;
use deno_resolver::file_fetcher::PermissionedFileFetcher;
use deno_resolver::file_fetcher::PermissionedFileFetcherOptions;
use deno_resolver::workspace::SpecifiedImportMap;
use deno_resolver::NodeResolverOptions;
use deno_semver::npm::NpmPackageReqReference;
use graph::ModuleGraphOptions;
use mappings::Mappings;
use mappings::SYNTHETIC_SPECIFIERS;
use mappings::SYNTHETIC_TEST_SPECIFIERS;
use node_resolver::analyze::NodeCodeTranslatorMode;
use node_resolver::NodeConditionOptions;
use polyfills::build_polyfill_file;
use polyfills::polyfills_for_target;
use polyfills::Polyfill;
pub use polyfills::PolyfillOverrides;
use specifiers::Specifiers;
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
pub use deno_graph::source::CacheSetting;
pub use deno_graph::source::LoadError;
pub use deno_graph::source::LoaderChecksum;

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
mod utils;
mod visitors;

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, Eq, PartialEq)]
pub struct OutputFile {
  pub file_path: PathBuf,
  pub file_text: String,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
  pub name: String,
  pub version: String,
  #[serde(default)]
  pub peer_dependency: bool,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq)]
pub struct TransformOutput {
  pub main: TransformOutputEnvironment,
  pub test: TransformOutputEnvironment,
  pub warnings: Vec<String>,
  /// Config file that was auto-discovered by searching upwards from the
  /// entry points (or the cwd when there are no local entry points).
  ///
  /// This is `None` when no config file was found, when one was explicitly
  /// provided, or when auto-discovery is disabled.
  pub discovered_config_file: Option<PathBuf>,
  /// Packages that provide the type declarations of a mapped dependency
  /// (ex. an `@types/` package specified by an `X-TypeScript-Types` header).
  pub types_dependencies: Vec<Dependency>,
}

#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
#[derive(Debug, PartialEq, Eq, Default)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageMappedSpecifier {
  /// Name being mapped to.
  pub name: String,
  /// Version of the specifier. Leave this blank to not have a
  /// dependency (ex. Node modules like "fs")
  pub version: Option<String>,
  /// Sub path of the npm package to use in the module specifier.
  pub sub_path: Option<String>,
  /// If this is suggested to be a peer dependency.
  #[serde(default)]
  pub peer_dependency: bool,
}

impl PackageMappedSpecifier {
  pub fn from_npm_specifier(npm_specifier: &NpmPackageReqReference) -> Self {
    Self {
      name: npm_specifier.req().name.to_string(),
      version: Some(npm_specifier.req().version_req.version_text().to_string()),
      sub_path: npm_specifier.sub_path().map(|s| s.to_string()),
      peer_dependency: false,
    }
  }

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
  #[serde(default)]
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
  ES2022 = 9,
  ES2023 = 10,
  Latest = 11,
}

pub struct TransformOptions {
  pub entry_points: Vec<ModuleSpecifier>,
  pub test_entry_points: Vec<ModuleSpecifier>,
  pub shims: Vec<Shim>,
  pub test_shims: Vec<Shim>,
  /// Maps specifiers to an npm package or module.
  pub specifier_mappings: HashMap<ModuleSpecifier, MappedSpecifier>,
  /// Version of ECMAScript that the final code will target.
  /// This controls whether certain polyfills should occur.
  pub target: ScriptTarget,
  /// Explicitly enables or disables polyfills by name, taking precedence
  /// over what `target` implies.
  pub polyfills: PolyfillOverrides,
  pub config_file: Option<ModuleSpecifier>,
  /// Disables auto-discovering a config file based on the entry points
  /// when no config file or import map is provided.
  ///
  /// Note this also disables discovering a package.json and deno.lock.
  pub no_config: bool,
  pub import_map: Option<ModuleSpecifier>,
  /// Errors when the deno lockfile would need to be updated in order to
  /// transform (ex. a dependency is not in it).
  ///
  /// Leave this `None` to use the `lock.frozen` setting in the deno.json file.
  pub frozen_lockfile: Option<bool>,
  pub cwd: PathBuf,
}

#[derive(Debug)]
struct ImportMapProvider(deno_cache_dir::file_fetcher::File);

#[async_trait::async_trait(?Send)]
impl SpecifiedImportMapProvider for ImportMapProvider {
  async fn get(&self) -> Result<Option<SpecifiedImportMap>, anyhow::Error> {
    let Some(value) = jsonc_parser::parse_to_serde_value(
      &String::from_utf8_lossy(&self.0.source),
      &jsonc_parser::ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: true,
        allow_trailing_commas: true,
      },
    )?
    else {
      return Ok(None);
    };
    Ok(Some(SpecifiedImportMap {
      base_url: self.0.url.clone(),
      value,
    }))
  }
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

pub async fn transform(
  sys: impl WorkspaceFactorySys,
  http_client: impl deno_cache_dir::file_fetcher::HttpClient + Clone + 'static,
  options: TransformOptions,
) -> Result<TransformOutput> {
  if options.entry_points.is_empty() {
    anyhow::bail!("at least one entry point must be specified");
  }

  let paths = options
    .entry_points
    .iter()
    .filter_map(|e| {
      deno_path_util::url_to_file_path(&deno_path_util::url_parent(e)).ok()
    })
    .collect::<Vec<_>>();
  let maybe_config_path = match &options.config_file {
    Some(config_file) => Some(deno_path_util::url_to_file_path(config_file)?),
    None => options
      .import_map
      .as_ref()
      .and_then(|import_map| deno_path_util::url_to_file_path(import_map).ok()),
  };
  let config_discovery = match maybe_config_path.as_ref() {
    Some(config_path) => ConfigDiscoveryOption::Path(config_path.clone()),
    None if options.no_config => ConfigDiscoveryOption::Disabled,
    None => {
      if paths.is_empty() {
        ConfigDiscoveryOption::DiscoverCwd
      } else {
        ConfigDiscoveryOption::Discover { start_paths: paths }
      }
    }
  };
  let is_auto_discovering = matches!(
    config_discovery,
    ConfigDiscoveryOption::Discover { .. } | ConfigDiscoveryOption::DiscoverCwd
  );

  let factory = deno_resolver::factory::WorkspaceFactory::new(
    sys,
    options.cwd,
    WorkspaceFactoryOptions {
      additional_config_file_names: &[],
      config_discovery,
      is_package_manager_subcommand: false,
      // force no node_modules directory so that we resolve package.json deps as npm specifiers
      node_modules_dir: Some(deno_config::deno_json::NodeModulesDirMode::None),
      node_modules_linker: None,
      no_npm: false,
      import_npm_lockfile: false,
      npm_process_state: None,
      root_node_modules_dir_override: None,
      vendor: None,
      frozen_lockfile: options.frozen_lockfile,
      lock_arg: None,
      lockfile_skip_write: true,
      maybe_custom_deno_dir_root: None,
      no_lock: false,
    },
  );
  let discovered_config_file = if is_auto_discovering {
    factory
      .workspace_directory()?
      .member_or_root_deno_json()
      .and_then(|c| deno_path_util::url_to_file_path(&c.specifier).ok())
  } else {
    None
  };
  let file_fetcher = PermissionedFileFetcher::new(
    NullBlobStore,
    Rc::new(factory.http_cache()?.clone()),
    http_client,
    Rc::new(deno_resolver::loader::MemoryFiles::default()),
    factory.sys().clone(),
    PermissionedFileFetcherOptions {
      allow_remote: true,
      cache_setting: deno_cache_dir::file_fetcher::CacheSetting::Use,
    },
  );
  let maybe_import_map_file = match options.import_map {
    Some(url) => Some(file_fetcher.fetch_bypass_permissions(&url).await?),
    None => None,
  };

  let resolver_factory = deno_resolver::factory::ResolverFactory::new(
    Rc::new(factory),
    ResolverFactoryOptions {
      is_cjs_resolution_mode:
        deno_resolver::cjs::IsCjsResolutionMode::ImplicitTypeCommonJs,
      npm_system_info: Default::default(),
      node_resolver_options: NodeResolverOptions {
        typescript_version: None,
        conditions: NodeConditionOptions {
          conditions: Vec::new(),
          import_conditions_override: None,
          require_conditions_override: None,
        },
        is_browser_platform: false,
        bundle_mode: false,
      },
      node_resolution_cache: None,
      package_json_cache: None,
      package_json_dep_resolution: Some(
        deno_resolver::workspace::PackageJsonDepResolution::Enabled,
      ),
      specified_import_map: maybe_import_map_file.map(|file| {
        Box::new(ImportMapProvider(file)) as Box<dyn SpecifiedImportMapProvider>
      }),
      unstable_sloppy_imports: true,
      on_mapped_resolution_diagnostic: None,
      compiler_options_overrides: CompilerOptionsOverrides {
        no_transpile: true,
        force_check_js: false,
        source_map_base: None,
        preserve_jsx: true,
        force_disable_verbatim_module_syntax: false,
      },
      newest_dependency_date: None,
      allow_json_imports: deno_resolver::loader::AllowJsonImports::Always,
      require_modules: Vec::new(),
      node_analysis_cache: None,
      node_code_translator_mode: NodeCodeTranslatorMode::ModuleLoader,
    },
  );
  let deno_resolver = resolver_factory.deno_resolver().await?;
  let cjs_tracker = resolver_factory.cjs_tracker()?.clone();
  let maybe_lockfile = resolver_factory
    .workspace_factory()
    .maybe_lockfile(&NullNpmPackageInfoProvider)
    .await?
    .cloned();

  let file_fetcher = Rc::new(file_fetcher);
  let loader = Rc::new(DenoGraphLoader::new(
    file_fetcher.clone(),
    resolver_factory
      .workspace_factory()
      .global_http_cache()?
      .clone(),
    resolver_factory.in_npm_package_checker()?.clone(),
    resolver_factory.workspace_factory().sys().clone(),
    DenoGraphLoaderOptions {
      file_header_overrides: Default::default(),
      include_npm_sources: false,
      permissions: None,
      reporter: None,
    },
  ));

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
      loader,
      resolver: deno_resolver.clone(),
      compiler_options_resolver: resolver_factory
        .compiler_options_resolver()?
        .clone(),
      cjs_tracker,
      maybe_lockfile,
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
  let types_dependencies =
    get_types_dependencies(&specifiers, &file_fetcher).await;
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
    searching_polyfills: polyfills_for_target(
      options.target,
      &options.polyfills,
    ),
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
    searching_polyfills: polyfills_for_target(
      options.target,
      &options.polyfills,
    ),
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
    .chain(specifiers.types.values().map(|d| &d.selected.specifier))
  {
    let module = module_graph.get(specifier);
    let env_context = if specifiers.test_modules.contains(specifier) {
      &mut test_env_context
    } else {
      &mut main_env_context
    };

    let file_text = match module {
      Module::Js(module) => {
        let parsed_source = module_graph.get_parsed_source(module)?;
        let text_changes = parsed_source
          .with_view(|program| -> Result<Vec<TextChange>> {
            let ignore_line_indexes = get_ignore_line_indexes(
              parsed_source.specifier().as_str(),
              program,
            );
            let top_level_decls =
              get_top_level_decls(program, parsed_source.top_level_context());
            warnings.extend(ignore_line_indexes.warnings);

            fill_polyfills(&mut FillPolyfillsParams {
              found_polyfills: &mut env_context.found_polyfills,
              searching_polyfills: &mut env_context.searching_polyfills,
              program,
              unresolved_context: parsed_source.unresolved_context(),
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
                  program,
                  unresolved_context: parsed_source.unresolved_context(),
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
              .extend(get_deno_comment_directive_text_changes(program));
            text_changes.extend(get_import_exports_text_changes(
              &GetImportExportsTextChangesParams {
                specifier,
                module_graph: &module_graph,
                mappings: &mappings,
                program,
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

        apply_text_changes(parsed_source.text(), text_changes)
      }
      Module::Json(module) => {
        format!("export default {};", strip_bom(&module.source.text).trim(),)
      }
      Module::Node(_)
      | Module::Npm(_)
      | Module::External(_)
      | Module::Wasm(_) => {
        bail!("Not implemented module kind for {}", module.specifier())
      }
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
  test_env_context
    .environment
    .dependencies
    .retain(|d| !main_env_context.environment.dependencies.contains(d));

  Ok(TransformOutput {
    main: main_env_context.environment,
    test: test_env_context.environment,
    warnings,
    discovered_config_file,
    types_dependencies,
  })
}

/// Provides npm package info when reading the lockfile.
///
/// dnt resolves npm specifiers via the specifier mappers rather than through
/// the module graph, and never writes the lockfile back, so the only time this
/// is consulted is when an older lockfile is migrated to the latest version in
/// memory. Returning default info for each requested package is sufficient.
struct NullNpmPackageInfoProvider;

#[async_trait::async_trait(?Send)]
impl deno_lockfile::NpmPackageInfoProvider for NullNpmPackageInfoProvider {
  async fn get_npm_package_info(
    &self,
    values: &[deno_semver::package::PackageNv],
  ) -> Result<
    Vec<deno_lockfile::Lockfile5NpmInfo>,
    Box<dyn std::error::Error + Send + Sync>,
  > {
    Ok(vec![Default::default(); values.len()])
  }
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
            get_relative_specifier(&file.file_path, polyfill_file_path)
          ),
        );
      }
    }
  }
  for polyfill in &env_context.found_polyfills {
    for dep in polyfill.dependencies() {
      if !env_context
        .environment
        .dependencies
        .iter()
        .any(|d| d.name == dep.name)
      {
        env_context.environment.dependencies.push(dep);
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
              peer_dependency: shim.package.peer_dependency,
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

/// Gets the packages that provide the type declarations of the mapped
/// dependencies, which are added to the dev dependencies.
///
/// A mapped module isn't part of the module graph, so the `X-TypeScript-Types`
/// header of each is resolved here. Failures are ignored because a mapped
/// module isn't required to be fetchable.
async fn get_types_dependencies<TSys: WorkspaceFactorySys>(
  specifiers: &Specifiers,
  file_fetcher: &PermissionedFileFetcher<
    NullBlobStore,
    TSys,
    impl deno_cache_dir::file_fetcher::HttpClient,
  >,
) -> Vec<Dependency> {
  let mappers = get_all_specifier_mappers();
  let mut types_packages = specifiers.types_packages.clone();
  let header_packages = futures::future::join_all(
    specifiers
      .main
      .mapped
      .keys()
      .chain(specifiers.test.mapped.keys())
      .filter(|s| matches!(s.scheme(), "http" | "https"))
      .map(|specifier| async {
        let types_specifier =
          get_types_header_specifier(file_fetcher, specifier).await?;
        get_types_package_for_specifier(&mappers, &types_specifier)
      }),
  )
  .await;
  for package in header_packages.into_iter().flatten() {
    types_packages.insert(package.name.clone(), package);
  }

  types_packages
    .into_values()
    .filter_map(|p| {
      Some(Dependency {
        name: p.name,
        version: p.version?,
        peer_dependency: false,
      })
    })
    .collect()
}

/// Gets the specifier of the declaration file in a module's
/// `X-TypeScript-Types` header.
async fn get_types_header_specifier<TSys: WorkspaceFactorySys>(
  file_fetcher: &PermissionedFileFetcher<
    NullBlobStore,
    TSys,
    impl deno_cache_dir::file_fetcher::HttpClient,
  >,
  specifier: &ModuleSpecifier,
) -> Option<ModuleSpecifier> {
  let file = file_fetcher
    .fetch_bypass_permissions(specifier)
    .await
    .ok()?;
  let types_header = file.maybe_headers?.remove("x-typescript-types")?;
  deno_path_util::resolve_url_or_path(&types_header, Path::new("")).ok()
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
          peer_dependency: entry.1.peer_dependency,
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

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_npm_mapper() {
    fn parse(specifier: &str) -> Option<PackageMappedSpecifier> {
      let npm_specifier = NpmPackageReqReference::from_str(specifier).ok()?;
      Some(PackageMappedSpecifier::from_npm_specifier(&npm_specifier))
    }

    assert_eq!(
      parse("npm:package"),
      Some(PackageMappedSpecifier {
        name: "package".to_string(),
        version: Some("*".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
    assert_eq!(
      parse("npm:package@^2.1"),
      Some(PackageMappedSpecifier {
        name: "package".to_string(),
        version: Some("^2.1".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
    assert_eq!(
      parse("npm:preact/hooks"),
      Some(PackageMappedSpecifier {
        name: "preact".to_string(),
        version: Some("*".to_string()),
        sub_path: Some("hooks".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      parse("npm:package/sub/path"),
      Some(PackageMappedSpecifier {
        name: "package".to_string(),
        version: Some("*".to_string()),
        sub_path: Some("sub/path".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      parse("npm:@scope/name/path/sub"),
      Some(PackageMappedSpecifier {
        name: "@scope/name".to_string(),
        version: Some("*".to_string()),
        sub_path: Some("path/sub".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      parse("npm:package@^2.1/sub_path"),
      Some(PackageMappedSpecifier {
        name: "package".to_string(),
        version: Some("^2.1".to_string()),
        sub_path: Some("sub_path".to_string()),
        peer_dependency: false
      })
    );
    assert_eq!(
      parse("npm:@project/name@2.1.3"),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("2.1.3".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
    assert_eq!(
      parse("npm:/@project/name@2.1.3"),
      Some(PackageMappedSpecifier {
        name: "@project/name".to_string(),
        version: Some("2.1.3".to_string()),
        sub_path: None,
        peer_dependency: false
      })
    );
  }
}
