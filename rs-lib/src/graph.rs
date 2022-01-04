// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::loader::get_all_specifier_mappers;
use crate::loader::Loader;
use crate::loader::SourceLoader;
use crate::parser::ScopeAnalysisParser;
use crate::specifiers::get_specifiers;
use crate::specifiers::Specifiers;
use crate::MappedSpecifier;
use anyhow::Result;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_graph::Dependency;
use deno_graph::EsModule;
use deno_graph::Module;
use deno_graph::Resolved;
use deno_graph::SyntheticModule;

#[derive(Clone, Copy)]
pub enum ModuleRef<'a> {
  Es(&'a EsModule),
  Synthetic(&'a SyntheticModule),
}

impl<'a> ModuleRef<'a> {
  pub fn as_es_module(&self) -> Option<&EsModule> {
    match self {
      ModuleRef::Es(m) => Some(m),
      ModuleRef::Synthetic(_) => None,
    }
  }

  pub fn specifier(&self) -> &ModuleSpecifier {
    match self {
      ModuleRef::Es(m) => &m.specifier,
      ModuleRef::Synthetic(m) => &m.specifier,
    }
  }

  pub fn media_type(&self) -> MediaType {
    match self {
      ModuleRef::Es(m) => m.media_type,
      ModuleRef::Synthetic(m) => m.media_type,
    }
  }

  pub fn maybe_dependencies(&self) -> Option<&'a BTreeMap<String, Dependency>> {
    match self {
      ModuleRef::Es(m) => Some(&m.dependencies),
      ModuleRef::Synthetic(_) => None,
    }
  }

  pub fn maybe_types_dependency(&self) -> Option<&'a (String, Resolved)> {
    match self {
      ModuleRef::Es(m) => m.maybe_types_dependency.as_ref(),
      ModuleRef::Synthetic(_) => None,
    }
  }

  pub fn source(&self) -> Cow<str> {
    match self {
      ModuleRef::Es(m) => Cow::Borrowed(m.source.as_str()),
      ModuleRef::Synthetic(m) => match m.maybe_source.as_ref() {
        Some(s) => Cow::Borrowed(s.as_str()),
        None => Cow::Owned(String::new()),
      },
    }
  }
}

pub struct ModuleGraphOptions<'a> {
  pub entry_points: Vec<ModuleSpecifier>,
  pub test_entry_points: Vec<ModuleSpecifier>,
  pub loader: Option<Box<dyn Loader>>,
  pub specifier_mappings: &'a HashMap<ModuleSpecifier, MappedSpecifier>,
  pub redirects: &'a HashMap<ModuleSpecifier, ModuleSpecifier>,
}

/// Wrapper around deno_graph::ModuleGraph.
pub struct ModuleGraph {
  graph: deno_graph::ModuleGraph,
}

impl ModuleGraph {
  pub async fn build_with_specifiers(
    options: ModuleGraphOptions<'_>,
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
      options.specifier_mappings,
      options.redirects,
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
        error_message.push_str(&error.to_string());
        if !error_message.contains(error.specifier().as_str()) {
          error_message.push_str(&format!(" ({})", error.specifier()));
        }
      }
      anyhow::bail!("{}", error_message);
    }

    let loader_specifiers = loader.into_specifiers();

    let not_found_redirects = options
      .redirects
      .keys()
      .filter(|s| !loader_specifiers.redirects.contains_key(s))
      .collect::<Vec<_>>();
    if !not_found_redirects.is_empty() {
      anyhow::bail!(
        "The following specifiers were indicated to be redirected, but were not found:\n{}",
        format_specifiers_for_message(not_found_redirects),
      );
    }

    let specifiers = get_specifiers(
      &options.entry_points,
      loader_specifiers,
      &graph,
      &graph.all_modules(),
    )?;

    let not_found_specifiers = options
      .specifier_mappings
      .keys()
      .filter(|s| !specifiers.has_mapped(s))
      .collect::<Vec<_>>();
    if !not_found_specifiers.is_empty() {
      anyhow::bail!(
        "The following specifiers were indicated to be mapped, but were not found:\n{}",
        format_specifiers_for_message(not_found_specifiers),
      );
    }

    Ok((graph, specifiers))
  }

  pub fn redirects(&self) -> &BTreeMap<ModuleSpecifier, ModuleSpecifier> {
    &self.graph.redirects
  }

  pub fn resolve(&self, specifier: &ModuleSpecifier) -> ModuleSpecifier {
    self.graph.resolve(specifier)
  }

  pub fn get(&self, specifier: &ModuleSpecifier) -> ModuleRef<'_> {
    let module = self.graph.get(specifier).unwrap_or_else(|| {
      panic!("Programming error. Did not find specifier: {}", specifier);
    });
    match module {
      Module::Es(m) => ModuleRef::Es(m),
      Module::Synthetic(m) => ModuleRef::Synthetic(m),
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
  }

  pub fn all_modules(&self) -> Vec<ModuleRef<'_>> {
    self
      .graph
      .modules()
      .into_iter()
      .map(ModuleRef::Es)
      .chain(
        self
          .graph
          .synthetic_modules()
          .into_iter()
          .map(ModuleRef::Synthetic),
      )
      .collect()
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
