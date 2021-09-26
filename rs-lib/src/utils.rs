// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use deno_ast::ModuleSpecifier;

pub fn get_relative_path(
  from: impl AsRef<Path>,
  to: impl AsRef<Path>,
) -> PathBuf {
  pathdiff::diff_paths(to, from.as_ref().parent().unwrap()).unwrap()
}

#[cfg(feature = "rust")]
pub fn url_to_file_path(module_specifier: &ModuleSpecifier) -> Result<PathBuf> {
  module_specifier.to_file_path().map_err(|_| anyhow::anyhow!("Error converting url to file path: {}", module_specifier.to_string()))
}

#[cfg(not(feature = "rust"))]
pub fn url_to_file_path(module_specifier: &ModuleSpecifier) -> Result<PathBuf> {
  assert!(module_specifier.scheme() == "file");
  let path_segments = module_specifier.path_segments().unwrap();
  let mut final_text = String::new();
  for segment in path_segments {
    if !final_text.is_empty() {
      final_text.push_str("/");
    }
    final_text.push_str(segment);
  }
  Ok(PathBuf::from(final_text))
}
