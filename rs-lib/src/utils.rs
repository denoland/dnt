// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;
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

/// Gets a unique file path given the provided file path
/// and the set of existing file paths. Inserts to the
/// set when finding a unique path.
pub fn get_unique_path(
  mut path: PathBuf,
  unique_set: &mut HashSet<String>,
) -> PathBuf {
  let original_path = path.clone();
  let mut count = 2;
  // case insensitive comparison for case insensitive file systems
  while !unique_set.insert(path.to_string_lossy().to_lowercase()) {
    path = path_with_stem_suffix(&original_path, &count.to_string());
    count += 1;
  }
  path
}

/// Gets a path with the specified file stem suffix.
pub fn path_with_stem_suffix(path: &Path, suffix: &str) -> PathBuf {
  if let Some(file_name) = path.file_name().map(|f| f.to_string_lossy()) {
    if let Some(file_stem) = path.file_stem().map(|f| f.to_string_lossy()) {
      if let Some(ext) = path.extension().map(|f| f.to_string_lossy()) {
        return path
          .with_file_name(format!("{}_{}.{}", file_stem, suffix, ext));
      }
    }

    path.with_file_name(format!("{}_{}", file_name, suffix))
  } else {
    path.with_file_name(suffix)
  }
}

/// Strips the byte order mark from the provided text if it exists.
pub fn strip_bom(text: &str) -> &str {
  if text.starts_with(BOM_CHAR) {
    &text[BOM_CHAR.len_utf8()..]
  } else {
    text
  }
}

/// Partitions the provided specifiers by specifiers that do not have a
/// parent specifier.
pub fn partition_by_root_specifiers<'a>(
  specifiers: impl Iterator<Item = &'a ModuleSpecifier>,
) -> Vec<(ModuleSpecifier, Vec<ModuleSpecifier>)> {
  let mut root_specifiers: Vec<(ModuleSpecifier, Vec<ModuleSpecifier>)> =
    Vec::new();
  for remote_specifier in specifiers {
    let mut found = false;
    for (root_specifier, specifiers) in root_specifiers.iter_mut() {
      if let Some(relative_url) = root_specifier.make_relative(remote_specifier)
      {
        // found a new root
        if relative_url.starts_with("../") {
          let end_ancestor_index =
            relative_url.len() - relative_url.trim_start_matches("../").len();
          *root_specifier = root_specifier
            .join(&relative_url[..end_ancestor_index])
            .unwrap();
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
      root_specifiers.push((root_specifier, vec![remote_specifier.clone()]));
    }
  }
  root_specifiers
}

#[cfg(test)]
mod test {
  use std::collections::HashSet;

  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn test_path_with_stem_suffix() {
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/"), "2"),
      PathBuf::from("/2")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test"), "2"),
      PathBuf::from("/test_2")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test.txt"), "2"),
      PathBuf::from("/test_2.txt")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test/subdir"), "2"),
      PathBuf::from("/test/subdir_2")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test/subdir.other.txt"), "2"),
      PathBuf::from("/test/subdir.other_2.txt")
    );
  }

  #[test]
  fn test_unique_path() {
    let mut paths = HashSet::new();
    assert_eq!(
      get_unique_path(PathBuf::from("/test"), &mut paths),
      PathBuf::from("/test")
    );
    assert_eq!(
      get_unique_path(PathBuf::from("/test"), &mut paths),
      PathBuf::from("/test_2")
    );
    assert_eq!(
      get_unique_path(PathBuf::from("/test"), &mut paths),
      PathBuf::from("/test_3")
    );
    assert_eq!(
      get_unique_path(PathBuf::from("/TEST"), &mut paths),
      PathBuf::from("/TEST_4")
    );
    assert_eq!(
      get_unique_path(PathBuf::from("/test.txt"), &mut paths),
      PathBuf::from("/test.txt")
    );
    assert_eq!(
      get_unique_path(PathBuf::from("/test.txt"), &mut paths),
      PathBuf::from("/test_2.txt")
    );
    assert_eq!(
      get_unique_path(PathBuf::from("/TEST.TXT"), &mut paths),
      PathBuf::from("/TEST_3.TXT")
    );
  }

  #[test]
  fn partition_by_root_specifiers_same_sub_folder() {
    run_partition_by_root_specifiers_test(
      vec![
        "https://deno.land/x/mod/A.ts",
        "https://deno.land/x/mod/other/A.ts",
      ],
      vec![(
        "https://deno.land/x/mod/",
        vec![
          "https://deno.land/x/mod/A.ts",
          "https://deno.land/x/mod/other/A.ts",
        ],
      )],
    );
  }

  #[test]
  fn partition_by_root_specifiers_different_sub_folder() {
    run_partition_by_root_specifiers_test(
      vec![
        "https://deno.land/x/mod/A.ts",
        "https://deno.land/x/other/A.ts",
      ],
      vec![(
        "https://deno.land/x/",
        vec![
          "https://deno.land/x/mod/A.ts",
          "https://deno.land/x/other/A.ts",
        ],
      )],
    );
  }

  #[test]
  fn partition_by_root_specifiers_different_hosts() {
    run_partition_by_root_specifiers_test(
      vec![
        "https://deno.land/mod/A.ts",
        "https://localhost/mod/A.ts",
        "https://other/A.ts",
      ],
      vec![
        ("https://deno.land/mod/", vec!["https://deno.land/mod/A.ts"]),
        ("https://localhost/mod/", vec!["https://localhost/mod/A.ts"]),
        ("https://other/", vec!["https://other/A.ts"]),
      ],
    );
  }

  fn run_partition_by_root_specifiers_test(
    input: Vec<&str>,
    expected: Vec<(&str, Vec<&str>)>,
  ) {
    let input = input
      .iter()
      .map(|s| ModuleSpecifier::parse(s).unwrap())
      .collect::<Vec<_>>();
    let output = partition_by_root_specifiers(input.iter());
    // the assertion is much easier to compare when everything is strings
    let output = output
      .into_iter()
      .map(|(s, vec)| {
        (
          s.to_string(),
          vec.into_iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )
      })
      .collect::<Vec<_>>();
    let expected = expected
      .into_iter()
      .map(|(s, vec)| {
        (
          s.to_string(),
          vec.into_iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        )
      })
      .collect::<Vec<_>>();
    assert_eq!(output, expected);
  }
}
