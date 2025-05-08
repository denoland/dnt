// Copyright 2018-2024 the Deno authors. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;

use crate::loader::get_all_specifier_mappers;
use crate::loader::Loader;
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
use deno_config::workspace::WorkspaceDirectory;
use deno_graph::CapturingModuleAnalyzer;
use deno_graph::EsParser;
use deno_graph::JsModule;
use deno_graph::Module;
use deno_graph::ParseOptions;
use deno_graph::ParsedSourceStore;
use deno_resolver::factory::WorkspaceFactorySys;
use deno_resolver::graph::DefaultDenoResolverRc;
use deno_resolver::npm::DenoInNpmPackageChecker;
use deno_resolver::workspace::ScopedJsxImportSourceConfig;
use sys_traits::impls::RealSys;

pub struct ModuleGraphOptions<'a, TSys: WorkspaceFactorySys> {
  pub entry_points: Vec<ModuleSpecifier>,
  pub test_entry_points: Vec<ModuleSpecifier>,
  pub loader: Rc<dyn Loader>,
  pub resolver: DefaultDenoResolverRc<TSys>,
  pub specifier_mappings: &'a HashMap<ModuleSpecifier, MappedSpecifier>,
  pub cjs_tracker:
    Rc<deno_resolver::cjs::CjsTracker<DenoInNpmPackageChecker, TSys>>,
  pub workspace_dir: Rc<WorkspaceDirectory>,
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
    let loader = SourceLoader::new(
      loader,
      get_all_specifier_mappers(),
      options.specifier_mappings,
    );
    let scoped_jsx_import_source_config =
      ScopedJsxImportSourceConfig::from_workspace_dir(&options.workspace_dir)?;
    let source_parser = ScopeAnalysisParser;
    let capturing_analyzer =
      CapturingModuleAnalyzer::new(Some(Box::new(source_parser)), None);
    let mut graph = deno_graph::ModuleGraph::new(deno_graph::GraphKind::All);
    let graph_resolver = resolver.as_graph_resolver(
      &options.cjs_tracker,
      &scoped_jsx_import_source_config,
    );
    graph
      .build(
        options
          .entry_points
          .iter()
          .chain(options.test_entry_points.iter())
          .map(|s| s.to_owned())
          .collect(),
        &loader,
        deno_graph::BuildOptions {
          is_dynamic: false,
          skip_dynamic_deps: false,
          imports: Default::default(),
          resolver: Some(&graph_resolver),
          locker: None,
          module_analyzer: &capturing_analyzer,
          reporter: None,
          npm_resolver: None,
          file_system: &RealSys,
          jsr_url_provider: Default::default(),
          executor: Default::default(),
          passthrough_jsr_specifiers: false,
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
      .filter(|s| !loader_specifiers.mapped_modules.contains_key(s))
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
      .filter(|s| !specifiers.has_mapped(s))
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
    self.graph.get(specifier).unwrap_or_else(|| {
      panic!("dnt bug - Did not find specifier: {}", specifier);
    })
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
        source: js_module.source.clone(),
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
