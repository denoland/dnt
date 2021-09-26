// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::path::PathBuf;

use deno_ast::ModuleSpecifier;
use deno_graph::ModuleGraph;

use crate::utils::url_to_file_path;

pub struct Mappings {
  inner: HashMap<ModuleSpecifier, PathBuf>,
}

impl Mappings {
  pub fn new(
    module_graph: &ModuleGraph,
    local_specifiers: &[ModuleSpecifier],
    remote_specifiers: &[ModuleSpecifier],
  ) -> Self {
    let mut mappings = HashMap::new();
    let base_dir = get_base_dir(local_specifiers);
    for specifier in local_specifiers.iter() {
      let file_path = url_to_file_path(specifier).unwrap();
      let relative_file_path = file_path.strip_prefix(&base_dir).unwrap();
      mappings.insert(specifier.clone(), relative_file_path.to_path_buf());
    }

    let mut root_remote_specifiers: Vec<(
      ModuleSpecifier,
      Vec<ModuleSpecifier>,
    )> = Vec::new();
    for remote_specifier in remote_specifiers.iter() {
      let mut found = false;
      for (root_specifier, specifiers) in root_remote_specifiers.iter_mut() {
        let result = root_specifier.make_relative(remote_specifier);
        if let Some(result) = result {
          // found a new root
          if result.starts_with("../") {
            // todo: improve, this was just laziness
            let mut new_root_specifier = root_specifier.clone();
            let mut result = result.as_str();
            while result.starts_with("../") {
              result = &result[3..];
              new_root_specifier = new_root_specifier.join("../").unwrap();
            }
            *root_specifier = new_root_specifier;
          }

          specifiers.push(remote_specifier.clone());
          found = true;
          break;
        }
      }
      if !found {
        root_remote_specifiers
          .push((remote_specifier.clone(), vec![remote_specifier.clone()]));
      }
    }

    for (i, (root, specifiers)) in
      root_remote_specifiers.into_iter().enumerate()
    {
      let base_dir = PathBuf::from(format!("deps/{}/", i.to_string()));
      for specifier in specifiers {
        let media_type = module_graph.get(&specifier).unwrap().media_type;
        let relative = root.make_relative(&specifier).unwrap();
        // todo: Handle urls that are directories on the server.. I think maybe use a special
        // file name and check for collisions (of any extension)
        let mut path = base_dir.join(relative);
        path.set_extension(&media_type.as_ts_extension()[1..]);
        mappings.insert(specifier, path);
      }
    }

    Mappings { inner: mappings }
  }

  pub fn get_file_path(&self, specifier: &ModuleSpecifier) -> &PathBuf {
    self.inner.get(specifier).unwrap()
  }
}

fn get_base_dir(specifiers: &[ModuleSpecifier]) -> PathBuf {
  // todo: should maybe error on windows when the files
  // span different drives...
  let mut base_dir = url_to_file_path(&specifiers[0])
    .unwrap()
    .to_path_buf()
    .parent()
    .unwrap()
    .to_path_buf();
  for specifier in specifiers {
    let file_path = url_to_file_path(specifier).unwrap();
    let parent_dir = file_path.parent().unwrap();
    if base_dir.starts_with(parent_dir) {
      base_dir = parent_dir.to_path_buf();
    }
  }
  base_dir
}
