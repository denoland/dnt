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

pub fn url_to_file_path(module_specifier: &ModuleSpecifier) -> Result<PathBuf> {
  // module_specifier.to_file_path() does not work in a cross platform way
  // and it does not work in Wasm
  assert!(module_specifier.scheme() == "file");
  let path_segments = module_specifier.path_segments().unwrap().collect::<Vec<_>>();
  let mut final_text = String::new();
  for segment in path_segments.iter() {
    if !final_text.is_empty() {
      final_text.push_str("/");
    }
    final_text.push_str(segment);
  }
  if !is_windows_path_segment(path_segments[0]) {
    final_text = format!("/{}", final_text);
  }
  Ok(PathBuf::from(final_text))
}

fn is_windows_path_segment(specifier: &str) -> bool {
  let mut chars = specifier.chars();

  let first_char = chars.next();
  if first_char.is_none() || !first_char.unwrap().is_ascii_alphabetic() {
    return false;
  }

  if chars.next() != Some(':') {
    return false;
  }

  chars.next().is_none()
}
