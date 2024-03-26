// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::io::ErrorKind;
use std::pin::Pin;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_graph::source::CacheSetting;
use deno_graph::source::LoaderChecksum;
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
    maybe_checksum: Option<LoaderChecksum>,
  ) -> Pin<Box<dyn Future<Output = Result<Option<LoadResponse>>> + 'static>> {
    Box::pin(async move {
      if specifier.scheme() == "file" {
        let file_path = url_to_file_path(&specifier)?;
        return match tokio::fs::read(file_path).await {
          Ok(bytes) => {
            if let Some(checksum) = maybe_checksum {
              checksum.check_source(&bytes)?;
            }
            Ok(Some(LoadResponse {
              specifier,
              content: bytes,
              headers: None,
            }))
          }
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
      let bytes = response.bytes().await?;

      if let Some(checksum) = maybe_checksum {
        checksum.check_source(&bytes)?;
      }

      Ok(Some(LoadResponse {
        specifier: final_url,
        content: bytes.into(),
        headers: Some(headers),
      }))
    })
  }
}
