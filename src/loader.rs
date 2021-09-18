// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

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

fn to_sorted(values: &HashSet<ModuleSpecifier>) -> Vec<ModuleSpecifier> {
    let mut values = values
      .iter()
      .map(ToOwned::to_owned)
      .collect::<Vec<_>>();
    values.sort();
    values
}

impl Loader for SourceLoader {
  fn load(
    &mut self,
    specifier: &ModuleSpecifier,
    _is_dynamic: bool,
  ) -> deno_graph::source::LoadFuture {
    if specifier.scheme() != "file" {
      println!("Skipping {}...", specifier);
      self.remote_specifiers.insert(specifier.clone());
      return Box::pin(future::ready((
        specifier.clone(),
        Ok(Some(LoadResponse {
          specifier: specifier.clone(),
          content: Arc::new("".to_string()),
          maybe_headers: None,
        })),
      )));
    }

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
  }
}
