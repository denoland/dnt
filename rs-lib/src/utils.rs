// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use deno_ast::parse_module;
use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;
use deno_ast::view::NodeTrait;
use deno_ast::view::Program;
use deno_ast::view::SpannedExt;
use deno_ast::ModuleSpecifier;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;

use crate::text_changes::apply_text_changes;
use crate::text_changes::TextChange;

pub const BOM_CHAR: char = '\u{FEFF}';

pub fn get_relative_specifier(
  from: impl AsRef<Path>,
  to: impl AsRef<Path>,
) -> String {
  let relative_path = get_relative_path(from, to).with_extension("js");
  let relative_path_str = relative_path
    .to_string_lossy()
    .to_string()
    .replace('\\', "/");

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
    path = path_with_stem_suffix(&original_path, &format!("_{}", count));
    count += 1;
  }
  path
}

/// Gets a path with the specified file stem suffix.
///
/// Ex. `file.ts` with suffix `_2` returns `file_2.ts`
pub fn path_with_stem_suffix(path: &Path, suffix: &str) -> PathBuf {
  if let Some(file_name) = path.file_name().map(|f| f.to_string_lossy()) {
    if let Some(file_stem) = path.file_stem().map(|f| f.to_string_lossy()) {
      if let Some(ext) = path.extension().map(|f| f.to_string_lossy()) {
        return if file_stem.to_lowercase().ends_with(".d") {
          path.with_file_name(format!(
            "{}{}.{}.{}",
            &file_stem[..file_stem.len() - ".d".len()],
            suffix,
            // maintain casing
            &file_stem[file_stem.len() - "d".len()..],
            ext
          ))
        } else {
          path.with_file_name(format!("{}{}.{}", file_stem, suffix, ext))
        };
      }
    }

    path.with_file_name(format!("{}{}", file_name, suffix))
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

/// Partitions the provided specifiers by the non-path and non-query parts of a specifier.
pub fn partition_by_root_specifiers<'a>(
  specifiers: impl Iterator<Item = &'a ModuleSpecifier>,
) -> BTreeMap<ModuleSpecifier, Vec<ModuleSpecifier>> {
  let mut root_specifiers: BTreeMap<ModuleSpecifier, Vec<ModuleSpecifier>> =
    Default::default();
  for remote_specifier in specifiers {
    let mut root_specifier = remote_specifier.clone();
    root_specifier.set_query(None);
    root_specifier.set_path("/");

    let specifiers = root_specifiers.entry(root_specifier).or_default();
    specifiers.push(remote_specifier.clone());
  }
  root_specifiers
}

pub fn prepend_statement_to_text(
  file_path: &Path,
  file_text: &mut String,
  statement_text: &str,
) {
  // It's not great to have to reparse the file for this. Perhaps there is a utility
  // function in swc or maybe add one to deno_ast for parsing out the leading comments
  let source = SourceTextInfo::from_string(std::mem::take(file_text));
  let parsed_module = parse_module(ParseParams {
    specifier: file_path.to_string_lossy().to_string(),
    capture_tokens: true,
    maybe_syntax: None,
    media_type: file_path.into(),
    scope_analysis: false,
    source: source.clone(),
  });
  match parsed_module {
    Ok(parsed_module) => parsed_module.with_view(|program| {
      let text_change =
        text_change_for_prepend_statement_to_text(&program, statement_text);
      *file_text =
        apply_text_changes(source.text().to_string(), vec![text_change]);
    }),
    Err(_) => {
      // should never happen... fallback...
      *file_text = format!("{}\n{}", statement_text, source.text_str(),);
    }
  }
}

pub fn text_change_for_prepend_statement_to_text(
  program: &Program,
  statement_text: &str,
) -> TextChange {
  let insert_pos = top_file_insert_pos(program);
  TextChange {
    span: Span::new(insert_pos, insert_pos, Default::default()),
    new_text: format!(
      "{}{}\n",
      if insert_pos == BytePos(0) { "" } else { "\n" },
      statement_text,
    ),
  }
}

fn top_file_insert_pos(program: &Program) -> BytePos {
  let mut pos = BytePos(0);
  for comment in program.leading_comments() {
    // insert before any @ts-ignore or @ts-expect
    if comment.text_fast(program).to_lowercase().contains("@ts-") {
      break;
    }
    pos = comment.hi();
  }
  pos
}

#[cfg(test)]
mod test {
  use std::collections::HashSet;

  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn test_path_with_stem_suffix() {
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/"), "_2"),
      PathBuf::from("/_2")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test"), "_2"),
      PathBuf::from("/test_2")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test.txt"), "_2"),
      PathBuf::from("/test_2.txt")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test/subdir"), "_2"),
      PathBuf::from("/test/subdir_2")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test/subdir.other.txt"), "_2"),
      PathBuf::from("/test/subdir.other_2.txt")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test.d.ts"), "_2"),
      PathBuf::from("/test_2.d.ts")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test.D.TS"), "_2"),
      PathBuf::from("/test_2.D.TS")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test.d.mts"), "_2"),
      PathBuf::from("/test_2.d.mts")
    );
    assert_eq!(
      path_with_stem_suffix(&PathBuf::from("/test.d.cts"), "_2"),
      PathBuf::from("/test_2.d.cts")
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
        "https://deno.land/",
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
        "https://deno.land/",
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
        "http://deno.land/B.ts",
        "https://deno.land:8080/C.ts",
        "https://localhost/mod/A.ts",
        "https://other/A.ts",
      ],
      vec![
        ("http://deno.land/", vec!["http://deno.land/B.ts"]),
        ("https://deno.land/", vec!["https://deno.land/mod/A.ts"]),
        (
          "https://deno.land:8080/",
          vec!["https://deno.land:8080/C.ts"],
        ),
        ("https://localhost/", vec!["https://localhost/mod/A.ts"]),
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
