// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;
use std::pin::Pin;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use futures::Future;

use crate::LoadResponse;
use crate::Loader;

pub struct DefaultLoader {}

impl DefaultLoader {
  pub fn new() -> Self {
    Self {}
  }
}

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
      let headers = response
        .headers()
        .into_iter()
        .filter_map(|(key, value)| match value.to_str() {
          Ok(value) => Some((key.to_string(), value.to_string())),
          Err(_) => None,
        })
        .collect();
      let text = response.text().await?;

      Ok(LoadResponse {
        content: text,
        maybe_headers: Some(headers),
      })
    })
  }
}
