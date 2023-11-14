// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::io::ErrorKind;
use std::pin::Pin;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_graph::source::CacheSetting;
use futures::Future;

use crate::utils::url_to_file_path;
use crate::LoadResponse;
use crate::Loader;

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
    _cache_setting: CacheSetting,
  ) -> Pin<Box<dyn Future<Output = Result<Option<LoadResponse>>> + 'static>> {
    Box::pin(async move {
      if specifier.scheme() == "file" {
        let file_path = url_to_file_path(&specifier)?;
        return match tokio::fs::read_to_string(file_path).await {
          Ok(result) => Ok(Some(LoadResponse {
            specifier,
            content: result,
            headers: None,
          })),
          Err(err) => {
            if err.kind() == ErrorKind::NotFound {
              Ok(None)
            } else {
              Err(err.into())
            }
          }
        };
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

      Ok(Some(LoadResponse {
        specifier: final_url,
        content: text,
        headers: Some(headers),
      }))
    })
  }
}
