// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use futures::future;
use futures::Future;

use crate::utils::url_to_file_path;

pub struct LoadResponse {
  pub maybe_headers: Option<HashMap<String, String>>,
  pub content: String,
}

pub trait Loader {
  fn read_file(
    &self,
    file_path: PathBuf,
  ) -> Pin<Box<dyn Future<Output = std::io::Result<String>> + 'static>>;
  fn make_request(
    &self,
    url: ModuleSpecifier,
  ) -> Pin<Box<dyn Future<Output = Result<LoadResponse>> + 'static>>;
}

#[cfg(feature = "rust")]
pub struct DefaultLoader {}

#[cfg(feature = "rust")]
impl DefaultLoader {
  pub fn new() -> Self {
    Self {}
  }
}

#[cfg(feature = "rust")]
impl Loader for DefaultLoader {
  fn read_file(
    &self,
    file_path: PathBuf,
  ) -> Pin<Box<dyn Future<Output = std::io::Result<String>> + 'static>> {
    Box::pin(tokio::fs::read_to_string(file_path))
  }

  fn make_request(
    &self,
    specifier: ModuleSpecifier,
  ) -> Pin<Box<dyn Future<Output = Result<LoadResponse>> + 'static>> {
    Box::pin(async move {
      let response = reqwest::get(specifier.clone()).await?;
      let text = response.text().await?;

      Ok(LoadResponse {
        content: text,
        maybe_headers: None,
      })
    })
  }
}

pub struct SourceLoader {
  loader: Arc<Box<dyn Loader>>,
  local_specifiers: HashSet<ModuleSpecifier>,
  remote_specifiers: HashSet<ModuleSpecifier>,
}

impl SourceLoader {
  pub fn new(loader: Box<dyn Loader>) -> Self {
    Self {
      loader: Arc::new(loader),
      local_specifiers: HashSet::new(),
      remote_specifiers: HashSet::new(),
    }
  }

  pub fn local_specifiers(&self) -> Vec<ModuleSpecifier> {
    to_sorted(&self.local_specifiers)
  }

  pub fn remote_specifiers(&self) -> Vec<ModuleSpecifier> {
    to_sorted(&self.remote_specifiers)
  }
}

impl deno_graph::source::Loader for SourceLoader {
  fn load(
    &mut self,
    specifier: &ModuleSpecifier,
    // todo: handle dynamic
    _is_dynamic: bool,
  ) -> deno_graph::source::LoadFuture {
    if specifier.scheme() == "https" || specifier.scheme() == "http" {
      println!("Downloading {}...", specifier);
      self.remote_specifiers.insert(specifier.clone());

      let loader = self.loader.clone();
      let specifier = specifier.clone();
      return Box::pin(async move {
        let resp = loader.make_request(specifier.clone()).await;
        (specifier.clone(), resp.map(|r| Some(deno_graph::source::LoadResponse {
          specifier,
          content: Arc::new(r.content),
          maybe_headers: r.maybe_headers,
        })))
      });
    } else if specifier.scheme() == "file" {
      println!("Loading {}...", specifier);
      self.local_specifiers.insert(specifier.clone());

      let file_path = url_to_file_path(specifier).unwrap();
      let loader = self.loader.clone();
      let specifier = specifier.clone();
      return Box::pin(async move {
        let file_text = loader.read_file(file_path).await;
        (
          specifier.clone(),
          match file_text {
            Ok(file_text) => Ok(Some(deno_graph::source::LoadResponse {
              specifier,
              content: Arc::new(file_text),
              maybe_headers: None,
            })),
            Err(err) => Err(anyhow::anyhow!("{}", err.to_string())),
          },
        )
      });
    } else {
      Box::pin(future::ready((
        specifier.clone(),
        Err(anyhow::format_err!("Unsupported scheme: {}", specifier)),
      )))
    }
  }
}

fn to_sorted(values: &HashSet<ModuleSpecifier>) -> Vec<ModuleSpecifier> {
  let mut values = values.iter().map(ToOwned::to_owned).collect::<Vec<_>>();
  values.sort();
  values
}
