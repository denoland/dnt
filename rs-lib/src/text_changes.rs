// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::cmp::Ordering;

use deno_ast::swc::common::Span;

pub struct TextChange {
  pub span: Span,
  pub new_text: String,
}

pub fn apply_text_changes(
  mut source: String,
  mut changes: Vec<TextChange>,
) -> String {
  changes.sort_by(|a, b| match a.span.lo.0.cmp(&b.span.lo.0) {
    // reverse order
    Ordering::Greater => Ordering::Less,
    Ordering::Less => Ordering::Greater,
    Ordering::Equal => Ordering::Equal,
  });

  for change in changes {
    source = format!(
      "{}{}{}",
      &source[..change.span.lo.0 as usize],
      change.new_text,
      &source[change.span.hi.0 as usize..],
    );
  }

  source
}
