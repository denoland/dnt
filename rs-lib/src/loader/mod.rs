// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

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
  pub resolved_node_specifiers: HashSet<ModuleSpecifier>,
}

pub struct SourceLoader<'a> {
  loader: Arc<Box<dyn Loader>>,
  specifiers: Arc<Mutex<LoaderSpecifiers>>,
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
      specifiers: Default::default(),
      specifier_mappers,
      specifier_mappings,
    }
  }

  pub fn into_specifiers(self) -> LoaderSpecifiers {
    match Arc::try_unwrap(self.specifiers) {
      Ok(specifiers) => specifiers.into_inner().unwrap(),
      Err(specifiers) => specifiers.lock().unwrap().clone(),
    }
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
        .lock()
        .unwrap()
        .mapped
        .insert(specifier.clone(), mapping.clone());
      // provide a dummy file so that this module can be analyzed later
      return get_dummy_module(specifier);
    }

    for mapper in self.specifier_mappers.iter() {
      if let Some(entry) = mapper.map(specifier) {
        self
          .specifiers
          .lock()
          .unwrap()
          .mapped
          .insert(specifier.clone(), entry);
        // provide a dummy file so that this module can be analyzed later
        return get_dummy_module(specifier);
      }
    }

    let loader = self.loader.clone();
    let specifier = specifier.clone();
    let specifiers = self.specifiers.clone();

    Box::pin(async move {
      // get the corresponding node runtime file
      if let Some((node_specifier, response)) =
        get_node_runtime_file(&**loader, &specifier).await
      {
        specifiers
          .lock()
          .unwrap()
          .resolved_node_specifiers
          .insert(node_specifier);
        return (specifier, response.map(Some));
      }

      let resp = loader.load(specifier.clone()).await;
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

async fn get_node_runtime_file(
  loader: &dyn Loader,
  specifier: &ModuleSpecifier,
) -> Option<(ModuleSpecifier, Result<deno_graph::source::LoadResponse>)> {
  // if this is a `*.deno.ts` file, check for the existence of a `*.node.ts` file
  let captures = match DENO_SUFFIX_RE.captures(specifier.path()) {
    Some(captures) => captures,
    None => return None,
  };
  let node_specifier = {
    let new_path = format!(
      "{}.node.{}",
      captures.get(1).unwrap().as_str(),
      captures.get(2).unwrap().as_str()
    );
    let mut specifier = specifier.clone();
    specifier.set_path(&new_path);
    specifier
  };
  let node_response = loader.load(node_specifier.clone()).await;
  match node_response {
    Ok(Some(r)) => Some((
      r.specifier.clone(),
      Ok(deno_graph::source::LoadResponse {
        specifier: r.specifier,
        content: Arc::new(r.content),
        maybe_headers: r.headers,
      }),
    )),
    Ok(None) => None,
    Err(err) => Some((node_specifier, Err(err))),
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
