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
pub use specifier_mappers::*;

use crate::MappedSpecifier;

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
  ) -> Pin<Box<dyn Future<Output = Result<LoadResponse>> + 'static>>;
}

pub struct LoaderSpecifiers {
  pub mapped: BTreeMap<ModuleSpecifier, MappedSpecifier>,
}

pub struct SourceLoader<'a> {
  #[allow(clippy::redundant_allocation)]
  loader: Arc<Box<dyn Loader>>,
  specifiers: LoaderSpecifiers,
  specifier_mappers: Vec<Box<dyn SpecifierMapper>>,
  specifier_mappings: Option<&'a HashMap<ModuleSpecifier, MappedSpecifier>>,
}

impl<'a> SourceLoader<'a> {
  pub fn new(
    loader: Box<dyn Loader>,
    specifier_mappers: Vec<Box<dyn SpecifierMapper>>,
    specifier_mappings: Option<&'a HashMap<ModuleSpecifier, MappedSpecifier>>,
  ) -> Self {
    Self {
      loader: Arc::new(loader),
      specifiers: LoaderSpecifiers {
        mapped: BTreeMap::new(),
      },
      specifier_mappers,
      specifier_mappings,
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
      .as_ref()
      .map(|m| m.get(specifier))
      .flatten()
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
        self.specifiers.mapped.insert(specifier.clone(), entry);
        // provide a dummy file so that this module can be analyzed later
        return get_dummy_module(specifier);
      }
    }

    let loader = self.loader.clone();
    let specifier = specifier.clone();

    Box::pin(async move {
      let resp = loader.load(specifier.clone()).await;
      (
        specifier.clone(),
        resp.map(|r| {
          Some(deno_graph::source::LoadResponse {
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
