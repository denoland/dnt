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
use deno_graph::ModuleGraphError;
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
    let jsr_specifier_mappings =
      JsrSpecifierMappings::new(options.specifier_mappings);
    let loader = SourceLoader::new(
      loader,
      get_all_specifier_mappers(),
      options.specifier_mappings,
      &jsr_specifier_mappings,
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
    // module errors don't include specifiers that failed to resolve, so walk
    // the graph for those as well. Otherwise they would be silently emitted
    // into the output as-is, which is broken for anything but an npm package.
    for error in graph
      .walk(
        graph.roots.iter(),
        deno_graph::WalkOptions {
          check_js: deno_graph::CheckJsOption::True,
          kind: deno_graph::GraphKind::All,
          follow_dynamic: true,
          prefer_fast_check_graph: false,
        },
      )
      .errors()
    {
      let range = match &error {
        // already reported above
        ModuleGraphError::ModuleError(_) => continue,
        ModuleGraphError::ResolutionError(error)
        | ModuleGraphError::TypesResolutionError(error) => {
          error.range().clone()
        }
      };
      if !error_message.is_empty() {
        error_message.push_str("\n\n");
      }
      write!(error_message, "{:#}\n    at {}", error, range).unwrap();
    }
    if !error_message.is_empty() {
      bail!("{}", error_message);
    }

    // when the lockfile is frozen, error instead of silently using
    // dependencies that aren't in it
    if let Some(lockfile) = &options.maybe_lockfile {
      lockfile
        .error_if_changed()
        .map_err(|err| anyhow::anyhow!("{}", err))?;
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
        !jsr_specifier_mappings
          .was_found(s, loader_specifiers.mapped_modules.keys())
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
      .filter(|s| {
        !jsr_specifier_mappings.was_found(
          s,
          specifiers
            .main
            .mapped
            .keys()
            .chain(specifiers.test.mapped.keys()),
        )
      })
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
    Ok(match self.mappings.graph_specifier(&specifier) {
      Some(specifier) => specifier,
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
pub struct JsrSpecifierMappings {
  by_name_and_sub_path: HashMap<JsrMappingKey, MappedSpecifier>,
}

impl JsrSpecifierMappings {
  pub fn new(mappings: &HashMap<ModuleSpecifier, MappedSpecifier>) -> Self {
    Self {
      by_name_and_sub_path: mappings
        .iter()
        .filter_map(|(specifier, mapping)| {
          Some((jsr_mapping_key(specifier)?, mapping.clone()))
        })
        .collect(),
    }
  }

  /// Gets the specifier to use in the module graph for a mapped `jsr:`
  /// specifier.
  pub fn graph_specifier(
    &self,
    specifier: &ModuleSpecifier,
  ) -> Option<ModuleSpecifier> {
    if !self
      .by_name_and_sub_path
      .contains_key(&jsr_mapping_key(specifier)?)
    {
      return None;
    }
    ModuleSpecifier::parse(&format!(
      "{}:{}",
      MAPPED_JSR_SCHEME,
      specifier.path()
    ))
    .ok()
  }

  /// Gets the mapping for a specifier in the module graph, filling in the
  /// version of the package from the specifier when the mapping doesn't
  /// specify one.
  pub fn get(&self, specifier: &ModuleSpecifier) -> Option<MappedSpecifier> {
    let specifier = from_graph_specifier(specifier)?;
    let mapping = self
      .by_name_and_sub_path
      .get(&jsr_mapping_key(&specifier)?)?;
    let MappedSpecifier::Package(package) = mapping else {
      return Some(mapping.clone());
    };
    let mut package = package.clone();
    if package.version.is_none() {
      package.version = jsr_version_req(&specifier);
    }
    Some(MappedSpecifier::Package(package))
  }

  /// Whether the graph contained a specifier that the provided mapping
  /// applied to.
  pub fn was_found<'a>(
    &self,
    mapping_specifier: &ModuleSpecifier,
    mut graph_specifiers: impl Iterator<Item = &'a ModuleSpecifier>,
  ) -> bool {
    let Some(key) = jsr_mapping_key(mapping_specifier) else {
      return graph_specifiers.any(|s| s == mapping_specifier);
    };
    graph_specifiers.any(|s| {
      from_graph_specifier(s)
        .and_then(|s| jsr_mapping_key(&s))
        .is_some_and(|s| s == key)
    })
  }
}

/// Scheme mapped `jsr:` specifiers have within the module graph.
const MAPPED_JSR_SCHEME: &str = "dnt-jsr";

/// A `jsr:` mapping is keyed on the package name and sub path so that the
/// version requirement doesn't need to be repeated in the mapping.
type JsrMappingKey = (String, Option<String>);

fn jsr_mapping_key(specifier: &ModuleSpecifier) -> Option<JsrMappingKey> {
  if specifier.scheme() != "jsr" {
    return None;
  }
  let req_ref = JsrPackageReqReference::from_specifier(specifier).ok()?;
  Some((
    req_ref.req().name.to_string(),
    req_ref.sub_path().map(ToOwned::to_owned),
  ))
}

fn jsr_version_req(specifier: &ModuleSpecifier) -> Option<String> {
  let req_ref = JsrPackageReqReference::from_specifier(specifier).ok()?;
  let version_text = req_ref.req().version_req.version_text();
  // no version requirement, so leave it up to the mapping
  if version_text == "*" {
    return None;
  }
  Some(version_text.to_string())
}

fn from_graph_specifier(
  specifier: &ModuleSpecifier,
) -> Option<ModuleSpecifier> {
  if specifier.scheme() != MAPPED_JSR_SCHEME {
    return None;
  }
  ModuleSpecifier::parse(&format!("jsr:{}", specifier.path())).ok()
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
  fn test_jsr_mappings_graph_specifier() {
    // the version requirement is ignored when matching, but kept on the
    // specifier so that the version can be used for the dependency
    run_test("jsr:@scope/name", Some("dnt-jsr:@scope/name"));
    run_test("jsr:@scope/name@^1.0.0", Some("dnt-jsr:@scope/name@^1.0.0"));
    run_test("jsr:@scope/other/sub", Some("dnt-jsr:@scope/other/sub"));
    // ...but the sub path is not ignored
    run_test("jsr:@scope/name/sub", None);
    run_test("jsr:@scope/other", None);
    // not mapped
    run_test("jsr:@scope/not-mapped", None);
    run_test("https://localhost/mod.ts", None);

    fn run_test(specifier: &str, expected: Option<&str>) {
      assert_eq!(
        mappings()
          .graph_specifier(&parse(specifier))
          .as_ref()
          .map(ModuleSpecifier::as_str),
        expected
      );
    }
  }

  #[test]
  fn test_jsr_mappings_get() {
    // the version of the specifier is used when the mapping has none
    run_test("dnt-jsr:@scope/name@^1.0.0", Some(Some("^1.0.0")));
    // ...but the mapping's version wins when it has one
    run_test("dnt-jsr:@scope/other@^1.0.0/sub", Some(Some("~2.0.0")));
    // no version anywhere means no dependency
    run_test("dnt-jsr:@scope/name", Some(None));
    // not mapped
    run_test("dnt-jsr:@scope/not-mapped", None);
    run_test("jsr:@scope/name", None);

    fn run_test(specifier: &str, expected: Option<Option<&str>>) {
      let mapping = mappings().get(&parse(specifier));
      assert_eq!(
        mapping.as_ref().map(|m| match m {
          MappedSpecifier::Package(p) => p.version.as_deref(),
          MappedSpecifier::Module(_) => unreachable!(),
        }),
        expected
      );
    }
  }

  #[test]
  fn test_jsr_mappings_was_found() {
    let mappings = mappings();

    // matched by package name and sub path, not the specifier
    assert!(mappings.was_found(
      &parse("jsr:@scope/name"),
      [parse("dnt-jsr:@scope/name@^1.0.0")].iter()
    ));
    assert!(!mappings.was_found(
      &parse("jsr:@scope/name"),
      [parse("dnt-jsr:@scope/name/sub")].iter()
    ));
    // non-jsr mappings are matched on the specifier
    assert!(mappings.was_found(
      &parse("https://localhost/mod.ts"),
      [parse("https://localhost/mod.ts")].iter()
    ));
    assert!(!mappings.was_found(
      &parse("https://localhost/mod.ts"),
      [parse("https://localhost/other.ts")].iter()
    ));
  }

  fn mappings() -> JsrSpecifierMappings {
    JsrSpecifierMappings::new(&HashMap::from([
      (parse("jsr:@scope/name"), mapping(None)),
      (parse("jsr:@scope/other/sub"), mapping(Some("~2.0.0"))),
      (parse("https://localhost/mod.ts"), mapping(None)),
    ]))
  }

  fn mapping(version: Option<&str>) -> MappedSpecifier {
    MappedSpecifier::Package(PackageMappedSpecifier {
      name: "package".to_string(),
      version: version.map(ToOwned::to_owned),
      sub_path: None,
      peer_dependency: false,
    })
  }

  fn parse(specifier: &str) -> ModuleSpecifier {
    ModuleSpecifier::parse(specifier).unwrap()
  }
}
