// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_graph::source::CacheSetting;
use futures::future;
use futures::Future;

#[cfg(feature = "tokio-loader")]
mod default_loader;
mod specifier_mappers;

#[cfg(feature = "tokio-loader")]
pub use default_loader::*;
pub use specifier_mappers::*;

use crate::MappedSpecifier;
use crate::PackageMappedSpecifier;

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

#[derive(Debug, Default, Clone)]
pub struct LoaderSpecifiers {
  pub mapped_packages: BTreeMap<ModuleSpecifier, PackageMappedSpecifier>,
  pub mapped_modules: HashMap<ModuleSpecifier, ModuleSpecifier>,
}

pub struct SourceLoader<'a> {
  loader: Rc<dyn Loader>,
  specifiers: LoaderSpecifiers,
  specifier_mappers: Vec<Box<dyn SpecifierMapper>>,
  specifier_mappings: &'a HashMap<ModuleSpecifier, MappedSpecifier>,
}

impl<'a> SourceLoader<'a> {
  pub fn new(
    loader: Rc<dyn Loader>,
    specifier_mappers: Vec<Box<dyn SpecifierMapper>>,
    specifier_mappings: &'a HashMap<ModuleSpecifier, MappedSpecifier>,
  ) -> Self {
    Self {
      loader,
      specifiers: Default::default(),
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
    _is_dynamic: bool,
    // todo: handle this for the new registry
    _cache_setting: CacheSetting,
  ) -> deno_graph::source::LoadFuture {
    let specifier = match self.specifier_mappings.get(specifier) {
      Some(MappedSpecifier::Package(mapping)) => {
        self
          .specifiers
          .mapped_packages
          .insert(specifier.clone(), mapping.clone());
        // provide a dummy file so that this module can be analyzed later
        return get_dummy_module(specifier);
      }
      Some(MappedSpecifier::Module(redirect)) => {
        self
          .specifiers
          .mapped_modules
          .insert(specifier.clone(), redirect.clone());
        redirect
      }
      None => {
        for mapper in self.specifier_mappers.iter() {
          if let Some(entry) = mapper.map(specifier) {
            self
              .specifiers
              .mapped_packages
              .insert(specifier.clone(), entry);
            // provide a dummy file so that this module can be analyzed later
            return get_dummy_module(specifier);
          }
        }
        specifier
      }
    };

    let loader = self.loader.clone();
    let specifier = specifier.to_owned();
    Box::pin(async move {
      let resp = loader.load(specifier.clone()).await;
      resp.map(|r| {
        r.map(|r| deno_graph::source::LoadResponse::Module {
          specifier: r.specifier,
          content: r.content.into(),
          maybe_headers: r.headers,
        })
      })
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
  Box::pin(future::ready(Ok(Some(
    deno_graph::source::LoadResponse::Module {
      specifier: specifier.clone(),
      content: "".into(),
      maybe_headers: Some(headers),
    },
  ))))
}
