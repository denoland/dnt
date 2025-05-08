// Copyright 2018-2024 the Deno authors. MIT license.

use std::io::ErrorKind;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_error::JsErrorBox;
use deno_graph::source::CacheSetting;
use deno_graph::source::LoadError;
use deno_graph::source::LoaderChecksum;
use deno_path_util::url_to_file_path;
use futures::Future;

use crate::LoadResponse;
use crate::Loader;

#[derive(Debug)]
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
  ) -> Pin<
    Box<dyn Future<Output = Result<Option<LoadResponse>, LoadError>> + 'static>,
  > {
    Box::pin(async move {
      if specifier.scheme() == "file" {
        let file_path = url_to_file_path(&specifier).map_err(|err| {
          LoadError::Other(Arc::new(JsErrorBox::from_err(err)))
        })?;
        return match std::fs::read(file_path) {
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
              Err(LoadError::Other(Arc::new(JsErrorBox::from_err(err))))
            }
          }
        };
      }

      let response = reqwest::get(specifier.clone()).await.map_err(|err| {
        LoadError::Other(Arc::new(JsErrorBox::generic(err.to_string())))
      })?;
      let headers = response
        .headers()
        .into_iter()
        .filter_map(|(key, value)| match value.to_str() {
          Ok(value) => Some((key.to_string(), value.to_string())),
          Err(_) => None,
        })
        .collect();
      let final_url = response.url().to_owned();
      let bytes = response.bytes().await.map_err(|err| {
        LoadError::Other(Arc::new(JsErrorBox::generic(err.to_string())))
      })?;

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
