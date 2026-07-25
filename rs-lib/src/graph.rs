// Copyright 2018-2024 the Deno authors. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

use crate::loader::get_all_specifier_mappers;
use crate::loader::SourceLoader;
use crate::parser::ScopeAnalysisParser;
use crate::specifiers::get_specifiers;
use crate::specifiers::Specifiers;
use crate::MappedSpecifier;

use anyhow::bail;
use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_ast::ParseDiagnostic;
use deno_ast::ParsedSource;
use deno_graph::ast::CapturingModuleAnalyzer;
use deno_graph::ast::EsParser;
use deno_graph::ast::ParseOptions;
use deno_graph::ast::ParsedSourceStore;
use deno_graph::source::NullModuleInfoCacher;
use deno_graph::source::ResolutionKind;
use deno_graph::source::ResolveError;
use deno_graph::source::Resolver;
use deno_graph::JsModule;
use deno_graph::Module;
use deno_graph::Range;
use deno_resolver::deno_json::CompilerOptionsResolver;
use deno_resolver::deno_json::JsxImportSourceConfigResolver;
use deno_resolver::factory::WorkspaceFactorySys;
use deno_resolver::graph::DefaultDenoResolverRc;
use deno_resolver::npm::DenoInNpmPackageChecker;
use deno_semver::jsr::JsrPackageReqReference;
use sys_traits::impls::RealSys;

pub struct ModuleGraphOptions<'a, TSys: WorkspaceFactorySys> {
  pub entry_points: Vec<ModuleSpecifier>,
  pub test_entry_points: Vec<ModuleSpecifier>,
  pub loader: Rc<dyn deno_graph::source::Loader>,
  pub resolver: DefaultDenoResolverRc<TSys>,
  pub specifier_mappings: &'a HashMap<ModuleSpecifier, MappedSpecifier>,
  pub compiler_options_resolver: Rc<CompilerOptionsResolver>,
  pub cjs_tracker:
    Rc<deno_resolver::cjs::CjsTracker<DenoInNpmPackageChecker, TSys>>,
  /// The project's deno lockfile, used to lock module versions and verify
  /// remote module checksums while building the graph.
  pub maybe_lockfile: Option<deno_resolver::lockfile::LockfileLockRc<TSys>>,
}

/// Wrapper around deno_graph::ModuleGraph.
pub struct ModuleGraph {
  graph: deno_graph::ModuleGraph,
  capturing_analyzer: CapturingModuleAnalyzer,
}

impl ModuleGraph {
  pub async fn build_with_specifiers<TSys: WorkspaceFactorySys>(
    options: ModuleGraphOptions<'_, TSys>,
  ) -> Result<(Self, Specifiers)> {
    let resolver = options.resolver;
    let loader = options.loader;
    // mapped jsr specifiers don't keep their scheme in the graph, so the
    // loader needs to look them up by the specifier they're resolved to
    let graph_specifier_mappings = options
      .specifier_mappings
      .iter()
      .map(|(k, v)| (mapped_specifier_key(k), v.clone()))
      .collect::<HashMap<_, _>>();
    let loader = SourceLoader::new(
      loader,
      get_all_specifier_mappers(),
      &graph_specifier_mappings,
    );
    let scoped_jsx_import_source_config =
      JsxImportSourceConfigResolver::from_compiler_options_resolver(
        &options.compiler_options_resolver,
      )?;
    let source_parser = ScopeAnalysisParser;
    let capturing_analyzer =
      CapturingModuleAnalyzer::new(Some(Box::new(source_parser)), None);
    let mut graph = deno_graph::ModuleGraph::new(deno_graph::GraphKind::All);
    // seed the graph with the locked module versions and redirects so the
    // same versions Deno resolved for the project are used
    if let Some(lockfile) = &options.maybe_lockfile {
      lockfile.fill_graph(&mut graph);
    }
    let mut locker = options
      .maybe_lockfile
      .as_ref()
      .map(|lockfile| lockfile.as_deno_graph_locker());
    let graph_resolver = resolver.as_graph_resolver(
      &options.cjs_tracker,
      &scoped_jsx_import_source_config,
      None,
      deno_resolver::graph::NpmTypesResolutionMode::FallbackToExecution,
    );
    let jsr_specifier_mappings =
      JsrSpecifierMappings::new(options.specifier_mappings);
    let graph_resolver = MappedJsrSpecifierResolver {
      inner: &graph_resolver,
      mappings: &jsr_specifier_mappings,
    };
    graph
      .build(
        options
          .entry_points
          .iter()
          .chain(options.test_entry_points.iter())
          .map(|s| s.to_owned())
          .collect(),
        Vec::new(),
        &loader,
        deno_graph::BuildOptions {
          is_dynamic: false,
          skip_dynamic_deps: false,
          resolver: Some(&graph_resolver),
          locker: locker
            .as_mut()
            .map(|l| l as &mut dyn deno_graph::source::Locker),
          module_analyzer: &capturing_analyzer,
          module_info_cacher: &NullModuleInfoCacher,
          reporter: None,
          npm_resolver: None,
          file_system: &RealSys,
          jsr_url_provider: Default::default(),
          jsr_version_resolver: Default::default(),
          jsr_metadata_store: None,
          executor: Default::default(),
          passthrough_jsr_specifiers: false,
          unstable_bytes_imports: false,
          unstable_text_imports: false,
          unstable_css_imports: false,
        },
      )
      .await;

    let mut error_message = String::new();
    for error in graph.module_errors() {
      if !error_message.is_empty() {
        error_message.push_str("\n\n");
      }
      if let Some(range) = error.maybe_referrer() {
        write!(error_message, "{:#}\n    at {}", error, range).unwrap();
      } else {
        write!(error_message, "{:#}", error).unwrap();
      }
      if !error_message.contains(error.specifier().as_str()) {
        error_message.push_str(&format!(" ({})", error.specifier()));
      }
    }
    if !error_message.is_empty() {
      bail!("{}", error_message);
    }

    let graph = Self {
      graph,
      capturing_analyzer,
    };

    let loader_specifiers = loader.into_specifiers();

    let not_found_module_mappings = options
      .specifier_mappings
      .iter()
      .filter_map(|(k, v)| match v {
        MappedSpecifier::Package(_) => None,
        MappedSpecifier::Module(_) => Some(k),
      })
      .filter(|s| {
        !loader_specifiers
          .mapped_modules
          .contains_key(&mapped_specifier_key(s))
      })
      .collect::<Vec<_>>();
    if !not_found_module_mappings.is_empty() {
      bail!(
        "The following specifiers were indicated to be mapped to a module, but were not found:\n{}",
        format_specifiers_for_message(not_found_module_mappings),
      );
    }

    let specifiers = get_specifiers(
      &options.entry_points,
      loader_specifiers,
      &graph,
      graph.all_modules(),
    )?;

    let not_found_package_specifiers = options
      .specifier_mappings
      .iter()
      .filter_map(|(k, v)| match v {
        MappedSpecifier::Package(_) => Some(k),
        MappedSpecifier::Module(_) => None,
      })
      .filter(|s| !specifiers.has_mapped(&mapped_specifier_key(s)))
      .collect::<Vec<_>>();
    if !not_found_package_specifiers.is_empty() {
      bail!(
        "The following specifiers were indicated to be mapped to a package, but were not found:\n{}",
        format_specifiers_for_message(not_found_package_specifiers),
      );
    }

    Ok((graph, specifiers))
  }

  pub fn redirects(&self) -> &BTreeMap<ModuleSpecifier, ModuleSpecifier> {
    &self.graph.redirects
  }

  pub fn resolve<'a>(
    &'a self,
    specifier: &'a ModuleSpecifier,
  ) -> &'a ModuleSpecifier {
    self.graph.resolve(specifier)
  }

  pub fn get(&self, specifier: &ModuleSpecifier) -> &Module {
    self.try_get(specifier).unwrap_or_else(|| {
      panic!("dnt bug - Did not find specifier: {}", specifier);
    })
  }

  pub fn try_get(&self, specifier: &ModuleSpecifier) -> Option<&Module> {
    self.graph.get(specifier)
  }

  pub fn get_parsed_source(
    &self,
    js_module: &JsModule,
  ) -> Result<ParsedSource, ParseDiagnostic> {
    match self
      .capturing_analyzer
      .get_parsed_source(&js_module.specifier)
    {
      Some(parsed_source) => Ok(parsed_source),
      None => self.capturing_analyzer.parse_program(ParseOptions {
        specifier: &js_module.specifier,
        source: js_module.source.text.clone(),
        media_type: js_module.media_type,
        scope_analysis: false,
      }),
    }
  }

  pub fn resolve_dependency(
    &self,
    value: &str,
    referrer: &ModuleSpecifier,
  ) -> Option<ModuleSpecifier> {
    self
      .graph
      .resolve_dependency(value, referrer, /* prefer_types */ false)
      .cloned()
      .or_else(|| {
        let value_lower = value.to_lowercase();
        if value_lower.starts_with("https://")
          || value_lower.starts_with("http://")
          || value_lower.starts_with("file://")
        {
          ModuleSpecifier::parse(value).ok()
        } else if value_lower.starts_with("./")
          || value_lower.starts_with("../")
        {
          referrer.join(value).ok()
        } else {
          None
        }
      })
      .filter(|s| !matches!(s.scheme(), "node"))
  }

  pub fn all_modules(&self) -> impl Iterator<Item = &Module> {
    self.graph.modules()
  }
}

/// Resolves `jsr:` specifiers that the user has provided a mapping for to
/// [`MAPPED_JSR_SCHEME`] so that they make it to the loader.
///
/// deno_graph resolves `jsr:` specifiers against the registry itself, so
/// without this the loader never sees the specifier the mapping is keyed on
/// and the package ends up vendored instead of mapped.
#[derive(Debug)]
struct MappedJsrSpecifierResolver<'a> {
  inner: &'a dyn Resolver,
  mappings: &'a JsrSpecifierMappings,
}

impl Resolver for MappedJsrSpecifierResolver<'_> {
  fn default_jsx_import_source(
    &self,
    referrer: &ModuleSpecifier,
  ) -> Option<String> {
    self.inner.default_jsx_import_source(referrer)
  }

  fn default_jsx_import_source_types(
    &self,
    referrer: &ModuleSpecifier,
  ) -> Option<String> {
    self.inner.default_jsx_import_source_types(referrer)
  }

  fn jsx_import_source_module(&self, referrer: &ModuleSpecifier) -> &str {
    self.inner.jsx_import_source_module(referrer)
  }

  fn resolve(
    &self,
    specifier_text: &str,
    referrer_range: &Range,
    kind: ResolutionKind,
  ) -> Result<ModuleSpecifier, ResolveError> {
    let specifier = self.inner.resolve(specifier_text, referrer_range, kind)?;
    Ok(match self.mappings.get(&specifier) {
      Some(mapped) => mapped.clone(),
      None => specifier,
    })
  }

  fn resolve_types(
    &self,
    specifier: &ModuleSpecifier,
  ) -> Result<Option<(ModuleSpecifier, Option<Range>)>, ResolveError> {
    self.inner.resolve_types(specifier)
  }
}

/// The user's `jsr:` specifier mappings stored by package name and sub path
/// so that a mapping is found no matter what version requirement the
/// specifier being resolved has.
#[derive(Debug, Default)]
struct JsrSpecifierMappings {
  by_name_and_sub_path: HashMap<(String, Option<String>), ModuleSpecifier>,
}

impl JsrSpecifierMappings {
  pub fn new(mappings: &HashMap<ModuleSpecifier, MappedSpecifier>) -> Self {
    let mut by_name_and_sub_path = HashMap::new();
    for specifier in mappings.keys() {
      if let Some(key) = jsr_name_and_sub_path(specifier) {
        by_name_and_sub_path.insert(key, mapped_specifier_key(specifier));
      }
    }
    Self {
      by_name_and_sub_path,
    }
  }

  pub fn get(&self, specifier: &ModuleSpecifier) -> Option<&ModuleSpecifier> {
    self
      .by_name_and_sub_path
      .get(&jsr_name_and_sub_path(specifier)?)
  }
}

/// Scheme mapped `jsr:` specifiers are resolved to within the module graph.
const MAPPED_JSR_SCHEME: &str = "dnt-jsr";

/// Gets the specifier a mapping key has within the module graph.
fn mapped_specifier_key(specifier: &ModuleSpecifier) -> ModuleSpecifier {
  if specifier.scheme() != "jsr" {
    return specifier.clone();
  }
  ModuleSpecifier::parse(&format!("{}:{}", MAPPED_JSR_SCHEME, specifier.path()))
    .unwrap_or_else(|_| specifier.clone())
}

fn jsr_name_and_sub_path(
  specifier: &ModuleSpecifier,
) -> Option<(String, Option<String>)> {
  if specifier.scheme() != "jsr" {
    return None;
  }
  let req_ref = JsrPackageReqReference::from_specifier(specifier).ok()?;
  Some((
    req_ref.req().name.to_string(),
    req_ref.sub_path().map(ToOwned::to_owned),
  ))
}

fn format_specifiers_for_message(
  mut specifiers: Vec<&ModuleSpecifier>,
) -> String {
  specifiers.sort();
  specifiers
    .into_iter()
    .map(|s| format!("  * {}", s))
    .collect::<Vec<_>>()
    .join("\n")
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::PackageMappedSpecifier;

  #[test]
  fn test_mapped_specifier_key() {
    // non-jsr specifiers keep their scheme
    run_test("https://localhost/mod.ts", "https://localhost/mod.ts");
    run_test("npm:package@^1.0.0", "npm:package@^1.0.0");
    // jsr specifiers get a scheme deno_graph won't resolve itself
    run_test("jsr:@scope/name", "dnt-jsr:@scope/name");
    run_test(
      "jsr:@scope/name@^1.0.0/sub",
      "dnt-jsr:@scope/name@^1.0.0/sub",
    );

    fn run_test(specifier: &str, expected: &str) {
      assert_eq!(
        mapped_specifier_key(&ModuleSpecifier::parse(specifier).unwrap())
          .as_str(),
        expected
      );
    }
  }

  #[test]
  fn test_jsr_specifier_mappings() {
    let mappings = JsrSpecifierMappings::new(&HashMap::from([
      (parse("jsr:@scope/name"), mapping()),
      (parse("jsr:@scope/other@^1.0.0/sub"), mapping()),
      (parse("https://localhost/mod.ts"), mapping()),
    ]));

    // the version requirement is ignored when matching
    assert_eq!(
      get(&mappings, "jsr:@scope/name"),
      Some("dnt-jsr:@scope/name")
    );
    assert_eq!(
      get(&mappings, "jsr:@scope/name@^1.0.0"),
      Some("dnt-jsr:@scope/name")
    );
    assert_eq!(
      get(&mappings, "jsr:@scope/other/sub"),
      Some("dnt-jsr:@scope/other@^1.0.0/sub")
    );
    // ...but the sub path is not
    assert_eq!(get(&mappings, "jsr:@scope/name/sub"), None);
    assert_eq!(get(&mappings, "jsr:@scope/other"), None);
    assert_eq!(get(&mappings, "jsr:@scope/not-mapped"), None);
    assert_eq!(get(&mappings, "https://localhost/mod.ts"), None);

    fn get<'a>(
      mappings: &'a JsrSpecifierMappings,
      specifier: &str,
    ) -> Option<&'a str> {
      mappings.get(&parse(specifier)).map(ModuleSpecifier::as_str)
    }

    fn parse(specifier: &str) -> ModuleSpecifier {
      ModuleSpecifier::parse(specifier).unwrap()
    }

    fn mapping() -> MappedSpecifier {
      MappedSpecifier::Package(PackageMappedSpecifier {
        name: "package".to_string(),
        version: None,
        sub_path: None,
        peer_dependency: false,
      })
    }
  }
}
