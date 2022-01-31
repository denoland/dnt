// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use deno_ast::ModuleSpecifier;

pub const BOM_CHAR: char = '\u{FEFF}';

pub fn get_relative_specifier(
  from: impl AsRef<Path>,
  to: impl AsRef<Path>,
) -> String {
  let relative_path = get_relative_path(from, to).with_extension("js");
  let relative_path_str = relative_path
    .to_string_lossy()
    .to_string()
    .replace("\\", "/");

  if relative_path_str.starts_with("../") || relative_path_str.starts_with("./")
  {
    relative_path_str
  } else {
    format!("./{}", relative_path_str)
  }
}

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
  let path_segments = module_specifier
    .path_segments()
    .unwrap()
    .collect::<Vec<_>>();
  let mut final_text = String::new();
  for segment in path_segments.iter() {
    if !final_text.is_empty() {
      final_text.push('/');
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

/// Strips the byte order mark from the provided text if it exists.
pub fn strip_bom(text: &str) -> &str {
  if text.starts_with(BOM_CHAR) {
    &text[BOM_CHAR.len_utf8()..]
  } else {
    text
  }
}

pub fn partition_by_root_specifiers(specifiers: &[ModuleSpecifier]) -> Vec<(ModuleSpecifier, Vec<ModuleSpecifier>)> {
  let mut root_specifiers: Vec<(
    ModuleSpecifier,
    Vec<ModuleSpecifier>,
  )> = Vec::new();
  for remote_specifier in specifiers {
    let mut found = false;
    for (root_specifier, specifiers) in root_specifiers.iter_mut() {
      if let Some(relative_url) =
        root_specifier.make_relative(remote_specifier)
      {
        // found a new root
        if relative_url.starts_with("../") {
          let end_ancestor_index = relative_url.len() - relative_url.trim_start_matches("../").len();
          *root_specifier = root_specifier.join(&relative_url[..end_ancestor_index]).unwrap();
        }

        specifiers.push(remote_specifier.clone());
        found = true;
        break;
      }
    }
    if !found {
      // get the specifier without the directory
      let root_specifier = remote_specifier
        .join("./")
        .unwrap_or_else(|_| remote_specifier.clone());
      root_specifiers
        .push((root_specifier, vec![remote_specifier.clone()]));
    }
  }
  root_specifiers
}

#[cfg(test)]
mod test {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn partition_by_root_specifiers_same_sub_folder() {
    run_partition_by_root_specifiers_test(vec![
      "https://deno.land/x/mod/A.ts",
      "https://deno.land/x/mod/other/A.ts",
    ], vec![
      (
        "https://deno.land/x/mod/",
        vec![
          "https://deno.land/x/mod/A.ts",
          "https://deno.land/x/mod/other/A.ts",
        ],
      ),
    ]);
  }

  #[test]
  fn partition_by_root_specifiers_different_sub_folder() {
    run_partition_by_root_specifiers_test(vec![
      "https://deno.land/x/mod/A.ts",
      "https://deno.land/x/other/A.ts",
    ], vec![
      (
        "https://deno.land/x/",
        vec![
          "https://deno.land/x/mod/A.ts",
          "https://deno.land/x/other/A.ts",
        ],
      ),
    ]);
  }

  #[test]
  fn partition_by_root_specifiers_different_hosts() {
    run_partition_by_root_specifiers_test(vec![
      "https://deno.land/mod/A.ts",
      "https://localhost/mod/A.ts",
      "https://other/A.ts",
    ], vec![
      (
        "https://deno.land/mod/",
        vec![
          "https://deno.land/mod/A.ts",
        ],
      ),
      (
        "https://localhost/mod/",
        vec![
          "https://localhost/mod/A.ts",
        ],
      ),
      (
        "https://other/",
        vec![
          "https://other/A.ts",
        ],
      ),
    ]);
  }

  fn run_partition_by_root_specifiers_test(input: Vec<&str>, expected: Vec<(&str, Vec<&str>)>) {
    let input = input.iter().map(|s| ModuleSpecifier::parse(s).unwrap()).collect::<Vec<_>>();
    let output = partition_by_root_specifiers(&input);
    // the assertion is much easier to compare when everything is strings
    let output = output.into_iter().map(|(s, vec)| (s.to_string(), vec.into_iter().map(|s| s.to_string()).collect::<Vec<_>>())).collect::<Vec<_>>();
    let expected = expected.into_iter().map(|(s, vec)| (s.to_string(), vec.into_iter().map(|s| s.to_string()).collect::<Vec<_>>())).collect::<Vec<_>>();
    assert_eq!(output, expected);
  }
}