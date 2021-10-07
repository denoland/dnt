// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_graph::ModuleGraph;
use regex::Regex;

use crate::utils::url_to_file_path;

lazy_static! {
  static ref HAS_EXTENSION_RE: Regex = Regex::new(r"\.[A-Za-z0-9]*$").unwrap();
}

pub struct Specifiers {
  pub local: Vec<ModuleSpecifier>,
  pub remote: Vec<ModuleSpecifier>,
  pub types: BTreeMap<ModuleSpecifier, ModuleSpecifier>,
  pub found_ignored: HashSet<ModuleSpecifier>,
}

pub struct Mappings {
  inner: HashMap<ModuleSpecifier, PathBuf>,
}

impl Mappings {
  pub fn new(
    module_graph: &ModuleGraph,
    specifiers: &Specifiers,
  ) -> Result<Self> {
    let mut mappings = HashMap::new();
    let base_dir = get_base_dir(&specifiers.local)?;
    for specifier in specifiers.local.iter() {
      let file_path = url_to_file_path(specifier)?;
      let relative_file_path = file_path.strip_prefix(&base_dir)?;
      mappings.insert(specifier.clone(), relative_file_path.to_path_buf());
    }

    let mut root_remote_specifiers: Vec<(
      ModuleSpecifier,
      Vec<(ModuleSpecifier, MediaType)>,
    )> = Vec::new();
    for remote_specifier in specifiers.remote.iter() {
      let media_type = module_graph
        .get(&remote_specifier)
        .ok_or_else(|| {
          anyhow::anyhow!(
            "Programming error. Could not find module for: {}",
            remote_specifier.to_string()
          )
        })?
        .media_type;
      let mut found = false;
      for (root_specifier, specifiers) in root_remote_specifiers.iter_mut() {
        if let Some(relative_url) =
          root_specifier.make_relative(remote_specifier)
        {
          // found a new root
          if relative_url.starts_with("../") {
            // todo: improve, this was just laziness
            let mut new_root_specifier = root_specifier.clone();
            let mut relative_url = relative_url.as_str();
            while relative_url.starts_with("../") {
              relative_url = &relative_url[3..];
              new_root_specifier = new_root_specifier.join("../").unwrap();
            }
            *root_specifier = new_root_specifier;
          }

          specifiers.push((remote_specifier.clone(), media_type));
          found = true;
          break;
        }
      }
      if !found {
        let root_specifier = remote_specifier
          .join("../")
          .unwrap_or_else(|_| remote_specifier.clone());
        root_remote_specifiers
          .push((root_specifier, vec![(remote_specifier.clone(), media_type)]));
      }
    }

    let mut mapped_filepaths_no_ext = HashSet::new();
    for (i, (root, specifiers)) in
      root_remote_specifiers.into_iter().enumerate()
    {
      let base_dir = PathBuf::from(format!("deps/{}/", i.to_string()));
      for (specifier, media_type) in specifiers {
        let relative = make_url_relative(&root, &specifier)?;
        let mut filepath_no_ext = base_dir.join(relative).with_extension("");
        let original_file_name = filepath_no_ext
          .file_name()
          .unwrap()
          .to_string_lossy()
          .to_string();
        let mut count = 2;
        while !mapped_filepaths_no_ext.insert(filepath_no_ext.clone()) {
          filepath_no_ext
            .set_file_name(format!("{}_{}", original_file_name, count));
          count += 1;
        }
        let file_path =
          filepath_no_ext.with_extension(&media_type.as_ts_extension()[1..]);
        mappings.insert(specifier, file_path);
      }
    }

    for (from, to) in specifiers.types.iter() {
      let file_path = mappings.get(&from).unwrap_or_else(|| {
        panic!("Already had from {} in map when mapping to {}.", from, to)
      });
      let new_file_path = file_path.with_extension("d.ts");
      if let Some(past_path) = mappings.insert(to.clone(), new_file_path) {
        panic!(
          "Already had path {} in map when mapping from {} to {}",
          past_path.display(),
          from,
          to
        );
      }
    }

    Ok(Mappings { inner: mappings })
  }

  pub fn get_file_path(&self, specifier: &ModuleSpecifier) -> &PathBuf {
    self.inner.get(specifier).unwrap_or_else(|| {
      panic!(
        "Programming error. Could not find file path for specifier: {}",
        specifier.to_string()
      )
    })
  }
}

fn make_url_relative(
  root: &ModuleSpecifier,
  url: &ModuleSpecifier,
) -> Result<String> {
  root.make_relative(&url).ok_or_else(|| {
    anyhow::anyhow!(
      "Error making url ({}) relative to root: {}",
      url.to_string(),
      root.to_string()
    )
  })
}

fn get_base_dir(specifiers: &[ModuleSpecifier]) -> Result<PathBuf> {
  // todo: should maybe error on windows when the files
  // span different drives...
  let mut base_dir = url_to_file_path(&specifiers[0])?
    .to_path_buf()
    .parent()
    .unwrap()
    .to_path_buf();
  for specifier in specifiers {
    let file_path = url_to_file_path(specifier)?;
    let parent_dir = file_path.parent().unwrap();
    if base_dir.starts_with(parent_dir) {
      base_dir = parent_dir.to_path_buf();
    }
  }
  Ok(base_dir)
}
