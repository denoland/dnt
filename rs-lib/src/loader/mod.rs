// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use futures::future;
use futures::Future;

#[cfg(feature = "tokio-loader")]
mod default_loader;
mod specifier_mappers;

#[cfg(feature = "tokio-loader")]
pub use default_loader::*;
use regex::Regex;
pub use specifier_mappers::*;

use crate::MappedSpecifier;

lazy_static! {
  static ref DENO_SUFFIX_RE: Regex =
    Regex::new(r"(?i)^(.*)\.deno\.([A-Za-z]+)$").unwrap();
}

#[cfg_attr(feature = "serialization", derive(serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(rename_all = "camelCase"))]
pub struct LoadResponse {
  /// The resolved specifier after re-directs.
  pub specifier: ModuleSpecifier,
  pub headers: Option<HashMap<String, String>>,
  pub content: String,
}

pub trait Loader {
  fn load(
    &self,
    url: ModuleSpecifier,
  ) -> Pin<Box<dyn Future<Output = Result<Option<LoadResponse>>> + 'static>>;
}

#[derive(Default, Clone)]
pub struct LoaderSpecifiers {
  pub mapped: BTreeMap<ModuleSpecifier, MappedSpecifier>,
  pub redirects: HashMap<ModuleSpecifier, ModuleSpecifier>,
}

pub struct SourceLoader<'a> {
  loader: Arc<Box<dyn Loader>>,
  specifiers: LoaderSpecifiers,
  specifier_mappers: Vec<Box<dyn SpecifierMapper>>,
  specifier_mappings: &'a HashMap<ModuleSpecifier, MappedSpecifier>,
  redirects: &'a HashMap<ModuleSpecifier, ModuleSpecifier>,
}

impl<'a> SourceLoader<'a> {
  pub fn new(
    loader: Box<dyn Loader>,
    specifier_mappers: Vec<Box<dyn SpecifierMapper>>,
    specifier_mappings: &'a HashMap<ModuleSpecifier, MappedSpecifier>,
    redirects: &'a HashMap<ModuleSpecifier, ModuleSpecifier>,
  ) -> Self {
    Self {
      loader: Arc::new(loader),
      specifiers: Default::default(),
      specifier_mappers,
      specifier_mappings,
      redirects,
    }
  }

  pub fn into_specifiers(self) -> LoaderSpecifiers {
    self.specifiers
  }
}

impl<'a> deno_graph::source::Loader for SourceLoader<'a> {
  fn load(
    &mut self,
    specifier: &ModuleSpecifier,
    // todo: handle dynamic
    _is_dynamic: bool,
  ) -> deno_graph::source::LoadFuture {
    if let Some(mapping) = self
      .specifier_mappings
      .get(specifier)
    {
      self
        .specifiers
        .mapped
        .insert(specifier.clone(), mapping.clone());
      // provide a dummy file so that this module can be analyzed later
      return get_dummy_module(specifier);
    }

    for mapper in self.specifier_mappers.iter() {
      if let Some(entry) = mapper.map(specifier) {
        self
          .specifiers
          .mapped
          .insert(specifier.clone(), entry);
        // provide a dummy file so that this module can be analyzed later
        return get_dummy_module(specifier);
      }
    }

    let loader = self.loader.clone();
    let specifier = specifier.clone();
    let load_specifier = if let Some(redirect) = self.redirects.get(&specifier).cloned() {
      self.specifiers.redirects.insert(specifier.clone(), redirect.clone());
      redirect
    } else {
      specifier.clone()
    };

    Box::pin(async move {
      let resp = loader.load(load_specifier.clone()).await;
      (
        specifier.clone(),
        resp.map(|r| {
          r.map(|r| deno_graph::source::LoadResponse {
            specifier: r.specifier,
            content: Arc::new(r.content),
            maybe_headers: r.headers,
          })
        }),
      )
    })
  }
}

fn get_dummy_module(
  specifier: &ModuleSpecifier,
) -> deno_graph::source::LoadFuture {
  let mut headers = HashMap::new();
  headers.insert(
    "content-type".to_string(),
    "application/javascript".to_string(),
  );
  Box::pin(future::ready((
    specifier.clone(),
    Ok(Some(deno_graph::source::LoadResponse {
      specifier: specifier.clone(),
      content: Arc::new(String::new()),
      maybe_headers: Some(headers),
    })),
  )))
}
