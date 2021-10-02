// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;

use anyhow::Result;
use futures::Future;

use d2n::ModuleSpecifier;
use d2n::LoadResponse;
use d2n::Loader;

#[derive(Clone)]
pub struct InMemoryLoader {
  local_files: HashMap<PathBuf, String>,
  remote_files: HashMap<ModuleSpecifier, Result<(String, Option<HashMap<String, String>>), String>>,
}

impl InMemoryLoader {
  pub fn new() -> Self {
    Self {
      local_files: HashMap::new(),
      remote_files: HashMap::new(),
    }
  }

  pub fn add_local_file(&mut self, path: impl AsRef<Path>, text: impl AsRef<str>) -> &mut Self {
    self.local_files.insert(path.as_ref().to_path_buf(), text.as_ref().to_string());
    self
  }

  pub fn add_remote_file(&mut self, specifier: impl AsRef<str>, text: impl AsRef<str>) -> &mut Self {
    self.remote_files.insert(ModuleSpecifier::parse(specifier.as_ref()).unwrap(), Ok((text.as_ref().to_string(), None)));
    self
  }

  pub fn add_remote_file_with_headers(&mut self, specifier: impl AsRef<str>, text: impl AsRef<str>, headers: &[(&str, &str)]) -> &mut Self {
    let headers = headers.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
    self.remote_files.insert(ModuleSpecifier::parse(specifier.as_ref()).unwrap(), Ok((text.as_ref().to_string(), Some(headers))));
    self
  }

  pub fn add_remote_file_with_error(&mut self, specifier: impl AsRef<str>, error_text: impl AsRef<str>) -> &mut Self {
    self.remote_files.insert(ModuleSpecifier::parse(specifier.as_ref()).unwrap(), Err(error_text.as_ref().to_string()));
    self
  }
}

impl Loader for InMemoryLoader {
  fn read_file(
    &self,
    file_path: PathBuf,
  ) -> Pin<Box<dyn Future<Output = std::io::Result<String>> + 'static>> {
    let result = self.local_files.get(&file_path).map(ToOwned::to_owned)
      .ok_or_else(|| std::io::ErrorKind::NotFound.into());
    Box::pin(futures::future::ready(result))
  }

  fn make_request(
    &self,
    specifier: ModuleSpecifier,
  ) -> Pin<Box<dyn Future<Output = Result<LoadResponse>> + 'static>> {
      let result = self.remote_files.get(&specifier).map(|result| {
        match result {
          Ok(result) => Ok(LoadResponse {
            content: result.0.clone(),
            maybe_headers: result.1.clone(),
          }),
          Err(err) => Err(err),
        }
      });
    let result = match result {
      Some(Ok(result)) => Ok(result),
      Some(Err(err)) => Err(anyhow::anyhow!("{}", err)),
      None => Err(anyhow::anyhow!("Not found.")),
    };
    Box::pin(futures::future::ready(result))
  }
}
