// Copyright 2018-2024 the Deno authors. MIT license.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_error::JsErrorBox;
use deno_graph::source::CacheSetting;
use deno_graph::source::LoadError;
use deno_graph::source::LoaderChecksum;
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
  pub content: Vec<u8>,
}

pub trait Loader: std::fmt::Debug {
  fn load(
    &self,
    url: ModuleSpecifier,
    cache_setting: CacheSetting,
    maybe_checksum: Option<LoaderChecksum>,
  ) -> Pin<
    Box<dyn Future<Output = Result<Option<LoadResponse>, LoadError>> + 'static>,
  >;
}

#[derive(Debug, Default, Clone)]
pub struct LoaderSpecifiers {
  pub mapped_packages: BTreeMap<ModuleSpecifier, PackageMappedSpecifier>,
  pub mapped_modules: HashMap<ModuleSpecifier, ModuleSpecifier>,
}

pub struct SourceLoader<'a> {
  loader: Rc<dyn Loader>,
  specifiers: RefCell<LoaderSpecifiers>,
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
    self.specifiers.take()
  }
}

impl deno_graph::source::Loader for SourceLoader<'_> {
  fn load(
    &self,
    specifier: &ModuleSpecifier,
    load_options: deno_graph::source::LoadOptions,
  ) -> deno_graph::source::LoadFuture {
    let specifier = match self.specifier_mappings.get(specifier) {
      Some(MappedSpecifier::Package(mapping)) => {
        self
          .specifiers
          .borrow_mut()
          .mapped_packages
          .insert(specifier.clone(), mapping.clone());
        // provide a dummy file so that this module can be analyzed later
        return get_dummy_module(specifier);
      }
      Some(MappedSpecifier::Module(redirect)) => {
        self
          .specifiers
          .borrow_mut()
          .mapped_modules
          .insert(specifier.clone(), redirect.clone());
        redirect
      }
      None => {
        for mapper in self.specifier_mappers.iter() {
          if let Some(entry) = mapper.map(specifier) {
            self
              .specifiers
              .borrow_mut()
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
      if specifier.scheme() == "node" {
        return Ok(Some(deno_graph::source::LoadResponse::External {
          specifier,
        }));
      }
      let resp = loader
        .load(
          specifier.clone(),
          load_options.cache_setting,
          load_options.maybe_checksum,
        )
        .await;
      resp
        .map(|r| {
          r.map(|r| deno_graph::source::LoadResponse::Module {
            specifier: r.specifier,
            content: r.content.into(),
            maybe_headers: r.headers,
            mtime: None,
          })
        })
        .map_err(|err| {
          deno_graph::source::LoadError::Other(Arc::new(JsErrorBox::from_err(
            err,
          )))
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
      content: b"".to_vec().into(),
      maybe_headers: Some(headers),
      mtime: None,
    },
  ))))
}
