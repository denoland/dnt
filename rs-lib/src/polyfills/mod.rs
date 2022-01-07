// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_ast::swc::common::SyntaxContext;
use deno_ast::view::Node;
use deno_ast::view::Program;

use crate::ScriptTarget;

mod error_cause;
mod object_has_own;
mod string_replace_all;
mod weak_ref;

pub trait Polyfill {
  fn use_for_target(&self, target: ScriptTarget) -> bool;
  fn visit_node(&self, node: Node, context: &PolyfillVisitContext<'_>) -> bool;
  fn get_file_text(&self) -> &'static str;
}

pub struct PolyfillVisitContext<'a> {
  pub program: &'a Program<'a>,
  pub top_level_context: SyntaxContext,
}

pub fn polyfills_for_target(target: ScriptTarget) -> Vec<Box<dyn Polyfill>> {
  all_polyfills()
    .into_iter()
    .filter(|p| p.use_for_target(target))
    .collect()
}

fn all_polyfills() -> Vec<Box<dyn Polyfill>> {
  vec![
    Box::new(object_has_own::ObjectHasOwnPolyfill),
    Box::new(error_cause::ErrorCausePolyfill),
    Box::new(string_replace_all::StringReplaceAllPolyfill),
    Box::new(weak_ref::WeakRefPolyfill),
  ]
}

pub fn build_polyfill_file(polyfills: &[Box<dyn Polyfill>]) -> Option<String> {
  if polyfills.is_empty() {
    return None;
  }

  let mut file_text = String::new();

  for polyfill in polyfills {
    file_text.push_str(polyfill.get_file_text());
  }

  Some(file_text)
}
