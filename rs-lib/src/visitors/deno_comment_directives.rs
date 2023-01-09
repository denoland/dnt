// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use deno_ast::swc::common::comments::Comment;
use deno_ast::view::*;
use deno_ast::RootNode;
use deno_ast::SourceRanged;
use deno_ast::SourceRangedForSpanned;
use deno_ast::SourceTextInfoProvider;
use deno_ast::TextChange;
use once_cell::sync::Lazy;
use regex::Regex;

// lifted from deno_graph
/// Matched the `@deno-types` pragma.
static DENO_TYPES_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r#"(?i)^\s*@deno-types\s*=\s*(?:["']([^"']+)["']|(\S+))"#).unwrap()
});
/// Matches a `/// <reference ... />` comment reference.
static TRIPLE_SLASH_REFERENCE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"(?i)^/\s*<reference\s.*?/>").unwrap());
/// Matches a types reference, which for JavaScript files indicates the
/// location of types to use when type checking a program that includes it as
/// a dependency.
static TYPES_REFERENCE_RE: Lazy<Regex> =
  Lazy::new(|| Regex::new(r#"(?i)\stypes\s*=\s*["']([^"']*)["']"#).unwrap());

pub fn get_deno_comment_directive_text_changes(
  program: &Program,
) -> Vec<TextChange> {
  let mut text_changes = Vec::new();

  // strip deno specific path triple slash references
  for comment in program.leading_comments_fast(program) {
    if TRIPLE_SLASH_REFERENCE_RE.is_match(&comment.text) {
      if let Some(captures) = TYPES_REFERENCE_RE.captures(&comment.text) {
        let specifier = captures.get(1).unwrap().as_str().to_lowercase();
        if specifier.starts_with("./")
          || specifier.starts_with("../")
          || specifier.starts_with("https://")
          || specifier.starts_with("http://")
        {
          text_changes.push(TextChange {
            new_text: String::new(),
            range: get_extended_comment_range(program, comment),
          });
        }
      }
    }
  }

  // strip all `@deno-types` comments
  for comment in program.comment_container().all_comments() {
    if DENO_TYPES_RE.is_match(&comment.text) {
      text_changes.push(TextChange {
        new_text: String::new(),
        range: get_extended_comment_range(program, comment),
      });
    }
  }

  text_changes
}

fn get_extended_comment_range(
  program: &Program,
  comment: &Comment,
) -> std::ops::Range<usize> {
  let text_info = program.text_info();
  let start_pos = text_info.range().start;
  let range = comment.range();
  let end_pos = range.end().as_byte_index(start_pos);
  range.start().as_byte_index(start_pos)..end_pos
}
