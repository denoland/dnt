// Copyright 2018-2024 the Deno authors. MIT license.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;
use deno_error::JsErrorBox;
use deno_graph::source::CacheSetting;
use deno_graph::source::LoaderChecksum;
use deno_node_transform::LoadError;
use deno_path_util::url_to_file_path;
use futures::Future;

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
    _cache_setting: CacheSetting,
    _maybe_checksum: Option<LoaderChecksum>,
  ) -> Pin<
    Box<dyn Future<Output = Result<Option<LoadResponse>, LoadError>> + 'static>,
  > {
    if specifier.scheme() == "file" {
      let file_path = url_to_file_path(&specifier).unwrap();
      let result = self.local_files.get(&file_path).map(ToOwned::to_owned);
      return Box::pin(async move {
        Ok(result.map(|result| LoadResponse {
          content: result.into_bytes(),
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
          content: result.0.clone().into(),
          headers: result.1.clone(),
        }),
        Err(err) => Err(err),
      });
    let result = match result {
      Some(Ok(result)) => Ok(Some(result)),
      Some(Err(err)) => Err(LoadError::Other(Arc::new(JsErrorBox::generic(
        format!("{}", err),
      )))),
      None => Ok(None),
    };
    Box::pin(futures::future::ready(result))
  }
}
