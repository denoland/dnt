// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;
use std::sync::Arc;

use deno_graph::source::LoadResponse;
use deno_graph::source::Loader;
use deno_graph::ModuleSpecifier;
use futures::future;

pub struct SourceLoader {
  local_specifiers: HashSet<ModuleSpecifier>,
  remote_specifiers: HashSet<ModuleSpecifier>,
}

impl SourceLoader {
  pub fn new() -> Self {
    Self {
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

impl Loader for SourceLoader {
  fn load(
    &mut self,
    specifier: &ModuleSpecifier,
    _is_dynamic: bool,
  ) -> deno_graph::source::LoadFuture {
    if specifier.scheme() == "https" || specifier.scheme() == "http" {
      println!("Downloading {}...", specifier);
      self.remote_specifiers.insert(specifier.clone());
      let specifier = specifier.clone();
      return Box::pin(async move {
        let resp = make_request(&specifier).await;
        (specifier, resp)
      });
    } else if specifier.scheme() == "file" {
      println!("Loading {}...", specifier);
      self.local_specifiers.insert(specifier.clone());

      let file_path = specifier.to_file_path().unwrap();
      let file_text = std::fs::read_to_string(file_path).unwrap();
      Box::pin(future::ready((
        specifier.clone(),
        Ok(Some(LoadResponse {
          specifier: specifier.clone(),
          content: Arc::new(file_text),
          maybe_headers: None,
        })),
      )))
    } else {
      Box::pin(future::ready((
        specifier.clone(),
        Err(anyhow::format_err!("Unsupported scheme: {}", specifier)),
      )))
    }
  }
}

async fn make_request(
  specifier: &ModuleSpecifier,
) -> anyhow::Result<Option<LoadResponse>> {
  let response = reqwest::get(specifier.clone()).await?;
  let text = response.text().await?;

  Ok(Some(LoadResponse {
    specifier: specifier.clone(),
    content: Arc::new(text.to_string()),
    maybe_headers: None,
  }))
}

fn to_sorted(values: &HashSet<ModuleSpecifier>) -> Vec<ModuleSpecifier> {
  let mut values = values.iter().map(ToOwned::to_owned).collect::<Vec<_>>();
  values.sort();
  values
}
