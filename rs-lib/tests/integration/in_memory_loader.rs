// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;

use anyhow::anyhow;
use anyhow::Result;
use futures::Future;

use deno_node_transform::url_to_file_path;
use deno_node_transform::LoadResponse;
use deno_node_transform::Loader;
use deno_node_transform::ModuleSpecifier;

type RemoteFileText = String;
type RemoteFileHeaders = Option<HashMap<String, String>>;
type RemoteFileResult = Result<(RemoteFileText, RemoteFileHeaders), String>;

#[derive(Clone)]
pub struct InMemoryLoader {
  local_files: HashMap<PathBuf, String>,
  remote_files: HashMap<ModuleSpecifier, RemoteFileResult>,
}

impl InMemoryLoader {
  pub fn new() -> Self {
    Self {
      local_files: HashMap::new(),
      remote_files: HashMap::new(),
    }
  }

  pub fn add_local_file(
    &mut self,
    path: impl AsRef<Path>,
    text: impl AsRef<str>,
  ) -> &mut Self {
    self
      .local_files
      .insert(path.as_ref().to_path_buf(), text.as_ref().to_string());
    self
  }

  pub fn add_remote_file(
    &mut self,
    specifier: impl AsRef<str>,
    text: impl AsRef<str>,
  ) -> &mut Self {
    self.remote_files.insert(
      ModuleSpecifier::parse(specifier.as_ref()).unwrap(),
      Ok((text.as_ref().to_string(), None)),
    );
    self
  }

  pub fn add_remote_file_with_headers(
    &mut self,
    specifier: impl AsRef<str>,
    text: impl AsRef<str>,
    headers: &[(&str, &str)],
  ) -> &mut Self {
    let headers = headers
      .iter()
      .map(|(key, value)| (key.to_string(), value.to_string()))
      .collect();
    self.remote_files.insert(
      ModuleSpecifier::parse(specifier.as_ref()).unwrap(),
      Ok((text.as_ref().to_string(), Some(headers))),
    );
    self
  }

  pub fn add_remote_file_with_error(
    &mut self,
    specifier: impl AsRef<str>,
    error_text: impl AsRef<str>,
  ) -> &mut Self {
    self.remote_files.insert(
      ModuleSpecifier::parse(specifier.as_ref()).unwrap(),
      Err(error_text.as_ref().to_string()),
    );
    self
  }
}

impl Loader for InMemoryLoader {
  fn load(
    &self,
    specifier: ModuleSpecifier,
  ) -> Pin<Box<dyn Future<Output = Result<Option<LoadResponse>>> + 'static>> {
    if specifier.scheme() == "file" {
      let file_path = url_to_file_path(&specifier).unwrap();
      let result = self.local_files.get(&file_path).map(ToOwned::to_owned);
      return Box::pin(async move {
        Ok(result.map(|result| LoadResponse {
          content: result,
          headers: None,
          specifier,
        }))
      });
    }
    let result = self
      .remote_files
      .get(&specifier)
      .map(|result| match result {
        Ok(result) => Ok(LoadResponse {
          specifier, // todo: test a re-direct
          content: result.0.clone(),
          headers: result.1.clone(),
        }),
        Err(err) => Err(err),
      });
    let result = match result {
      Some(Ok(result)) => Ok(Some(result)),
      Some(Err(err)) => Err(anyhow!("{}", err.to_string())),
      None => Ok(None),
    };
    Box::pin(futures::future::ready(result))
  }
}
