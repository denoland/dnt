// Copyright 2018-2024 the Deno authors. MIT license.

use deno_ast::view::Node;
use deno_ast::SourceRanged;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct ErrorCausePolyfill;

impl Polyfill for ErrorCausePolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    true
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    if let Node::MemberExpr(expr) = node {
      // very simple detection as we don't have type checking
      if expr.prop.text_fast(context.program) == "cause" {
        return true;
      }
    }
    false
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/esnext.error-cause.ts")
  }
}
