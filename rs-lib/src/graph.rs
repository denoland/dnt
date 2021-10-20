use std::collections::HashSet;

use crate::loader::get_all_specifier_mappers;
use crate::loader::Loader;
use crate::loader::SourceLoader;
use crate::parser::ScopeAnalysisParser;
use crate::specifiers::get_specifiers;
use crate::specifiers::Specifiers;
use anyhow::Result;
use deno_ast::ModuleSpecifier;

pub struct ModuleGraphOptions<'a> {
  pub entry_points: Vec<ModuleSpecifier>,
  pub test_entry_points: Vec<ModuleSpecifier>,
  pub loader: Option<Box<dyn Loader>>,
  pub ignored_specifiers: Option<&'a HashSet<ModuleSpecifier>>,
}

/// Wrapper around deno_graph::ModuleGraph.
pub struct ModuleGraph {
  graph: deno_graph::ModuleGraph,
}

impl ModuleGraph {
  pub async fn build_with_specifiers<'a>(
    options: ModuleGraphOptions<'a>,
  ) -> Result<(Self, Specifiers)> {
    let mut loader = SourceLoader::new(
      options.loader.unwrap_or_else(|| {
        #[cfg(feature = "tokio-loader")]
        return Box::new(crate::loader::DefaultLoader::new());
        #[cfg(not(feature = "tokio-loader"))]
        panic!("You must provide a loader or use the 'tokio-loader' feature.")
      }),
      // todo: support configuring this in the future
      get_all_specifier_mappers(),
      options.ignored_specifiers,
    );
    let source_parser = ScopeAnalysisParser::new();
    let graph = Self {
      graph: deno_graph::create_graph(
        options
          .entry_points
          .iter()
          .chain(options.test_entry_points.iter())
          .map(ToOwned::to_owned)
          .collect(),
        false,
        None,
        &mut loader,
        None,
        None,
        Some(&source_parser),
      )
      .await,
    };

    let errors = graph.graph.errors().into_iter().collect::<Vec<_>>();
    if !errors.is_empty() {
      let mut error_message = String::new();
      for error in errors {
        if !error_message.is_empty() {
          error_message.push_str("\n\n");
        }
        error_message.push_str(&format!(
          "{} ({})",
          error.to_string(),
          error.specifier()
        ));
      }
      anyhow::bail!("{}", error_message);
    }

    let loader_specifiers = loader.into_specifiers();
    let specifiers = get_specifiers(
      &options.entry_points,
      loader_specifiers,
      &graph,
      &graph.graph.modules(),
    )?;

    if let Some(ignored_specifiers) = options.ignored_specifiers {
      let mut not_found_specifiers = ignored_specifiers
        .iter()
        .filter(|s| !specifiers.has_ignored_or_mapped(s))
        .collect::<Vec<_>>();
      if !not_found_specifiers.is_empty() {
        not_found_specifiers.sort();
        anyhow::bail!(
        "The following specifiers were indicated to be mapped, but were not found:\n{}",
        not_found_specifiers.into_iter().map(|s| format!("  * {}", s)).collect::<Vec<_>>().join("\n"),
      );
      }
    }

    Ok((graph, specifiers))
  }

  pub fn get(&self, specifier: &ModuleSpecifier) -> &deno_graph::Module {
    self.graph.get(specifier).unwrap_or_else(|| {
      panic!("Programming error. Did not find specifier: {}", specifier);
    })
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
          ModuleSpecifier::parse(&value).ok()
        } else if value_lower.starts_with("./")
          || value_lower.starts_with("../")
        {
          referrer.join(&value).ok()
        } else {
          None
        }
      })
  }
}
