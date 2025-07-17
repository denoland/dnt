// Copyright 2018-2024 the Deno authors. MIT license.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use deno_cache_dir::file_fetcher::HeaderMap;
use deno_cache_dir::file_fetcher::HeaderName;
use deno_cache_dir::file_fetcher::SendError;
use deno_cache_dir::file_fetcher::SendResponse;

use deno_node_transform::ModuleSpecifier;
use sys_traits::impls::InMemorySys;
use sys_traits::EnvSetCurrentDir;
use sys_traits::EnvSetVar;
use sys_traits::FsCreateDirAll;
use sys_traits::FsWrite;

type RemoteFileText = String;
type RemoteFileHeaders = Option<HashMap<String, String>>;
type RemoteFileResult = Result<(RemoteFileText, RemoteFileHeaders), String>;

#[derive(Debug, Clone)]
pub struct InMemoryLoader {
  pub sys: InMemorySys,
  remote_files: HashMap<ModuleSpecifier, RemoteFileResult>,
}

impl InMemoryLoader {
  pub fn new() -> Self {
    let sys = InMemorySys::default();
    let deno_dir_folder = if cfg!(windows) { "C:/.deno" } else { "/.deno" };
    sys.env_set_var("DENO_DIR", deno_dir_folder);
    sys.fs_create_dir_all(deno_dir_folder).unwrap();
    if cfg!(windows) {
      sys.env_set_current_dir("C:\\").unwrap();
    }
    Self {
      sys,
      remote_files: HashMap::new(),
    }
  }

  pub fn add_local_file(
    &mut self,
    path: impl AsRef<str>,
    text: impl AsRef<str>,
  ) -> &mut Self {
    let path = path.as_ref();
    let path = if cfg!(windows) && path.starts_with("/") {
      PathBuf::from(format!("C:{}", path))
    } else {
      PathBuf::from(path)
    };
    let parent_dir = path.parent().unwrap();
    self.sys.fs_create_dir_all(parent_dir).unwrap();
    self.sys.fs_write(path, text.as_ref().as_bytes()).unwrap();
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

fn to_headers(src: HashMap<String, String>) -> HeaderMap {
  let mut h = HeaderMap::with_capacity(src.len());
  for (k, v) in src {
    let name = HeaderName::try_from(k.as_str()).unwrap();
    let value =
      deno_cache_dir::file_fetcher::HeaderValue::from_str(&v).unwrap();
    h.insert(name, value);
  }
  h
}

#[async_trait::async_trait(?Send)]
impl deno_cache_dir::file_fetcher::HttpClient for InMemoryLoader {
  async fn send_no_follow(
    &self,
    specifier: &ModuleSpecifier,
    _headers: HeaderMap,
  ) -> Result<SendResponse, SendError> {
    let result = self
      .remote_files
      .get(&specifier)
      .map(|result| match result {
        Ok(result) => Ok(SendResponse::Success(
          to_headers(result.1.clone().unwrap_or_default()),
          result.0.clone().into_bytes().into(),
        )),
        Err(err) => Err(SendError::Failed(err.clone().into())),
      });
    match result {
      Some(result) => result,
      None => Err(SendError::NotFound),
    }
  }
}
