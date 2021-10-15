// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use deno_ast::swc::common::comments::Comment;
use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::view::*;
use regex::Regex;

use crate::text_changes::TextChange;

// lifted from deno_graph
lazy_static! {
  /// Matched the `@deno-types` pragma.
  static ref DENO_TYPES_RE: Regex =
    Regex::new(r#"(?i)^\s*@deno-types\s*=\s*(?:["']([^"']+)["']|(\S+))"#)
      .unwrap();
  /// Matches a `/// <reference ... />` comment reference.
  static ref TRIPLE_SLASH_REFERENCE_RE: Regex =
    Regex::new(r"(?i)^/\s*<reference\s.*?/>").unwrap();
  /// Matches a types reference, which for JavaScript files indicates the
  /// location of types to use when type checking a program that includes it as
  /// a dependency.
  static ref TYPES_REFERENCE_RE: Regex =
    Regex::new(r#"(?i)\stypes\s*=\s*["']([^"']*)["']"#).unwrap();
}

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
            span: get_extended_comment_span(program, comment),
          });
        }
      }
    }
  }

  // strip all `@deno-types` comments
  for comment in program.comment_container().unwrap().all_comments() {
    if DENO_TYPES_RE.is_match(&comment.text) {
      text_changes.push(TextChange {
        new_text: String::new(),
        span: get_extended_comment_span(program, comment),
      });
    }
  }

  text_changes
}

fn get_extended_comment_span(program: &Program, comment: &Comment) -> Span {
  let file_text = program.source_file().unwrap().text();
  let span = comment.span();
  let end_pos = get_next_non_whitespace_pos(file_text, span.hi.0 as usize);
  Span::new(span.lo, BytePos(end_pos as u32), Default::default())
}

fn get_next_non_whitespace_pos(text: &str, start_pos: usize) -> usize {
  let mut end_pos = start_pos;
  for (i, c) in text[start_pos..].char_indices() {
    if !c.is_whitespace() {
      break;
    }
    end_pos = start_pos + i + c.len_utf8();
  }
  end_pos
}
