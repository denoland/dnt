// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::pin::Pin;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use futures::Future;

use crate::LoadResponse;
use crate::Loader;
use crate::utils::url_to_file_path;

pub struct DefaultLoader {}

impl DefaultLoader {
  pub fn new() -> Self {
    Self {}
  }
}

impl Loader for DefaultLoader {
  fn load(
    &self,
    specifier: ModuleSpecifier,
  ) -> Pin<Box<dyn Future<Output = Result<LoadResponse>> + 'static>> {
    Box::pin(async move {
      if specifier.scheme() == "file" {
        let file_path = url_to_file_path(&specifier)?;
        let result = tokio::fs::read_to_string(file_path).await?;
        return Ok(LoadResponse {
          specifier,
          content: result,
          headers: None,
        });
      }

      let response = reqwest::get(specifier.clone()).await?;
      let headers = response
        .headers()
        .into_iter()
        .filter_map(|(key, value)| match value.to_str() {
          Ok(value) => Some((key.to_string(), value.to_string())),
          Err(_) => None,
        })
        .collect();
      let final_url = response.url().to_owned();
      let text = response.text().await?;

      Ok(LoadResponse {
        specifier: final_url,
        content: text,
        headers: Some(headers),
      })
    })
  }
}
