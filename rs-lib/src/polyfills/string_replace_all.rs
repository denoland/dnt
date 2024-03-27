// Copyright 2018-2024 the Deno authors. MIT license.

use deno_ast::view::Callee;
use deno_ast::view::Expr;
use deno_ast::view::Node;
use deno_ast::SourceRanged;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct StringReplaceAllPolyfill;

impl Polyfill for StringReplaceAllPolyfill {
  fn use_for_target(&self, target: ScriptTarget) -> bool {
    (target as u32) < (ScriptTarget::ES2021 as u32)
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    if let Node::CallExpr(expr) = node {
      if let Callee::Expr(Expr::Member(callee)) = expr.callee {
        if expr.args.len() == 2
          && callee.prop.text_fast(context.program) == "replaceAll"
        {
          return true;
        }
      }
    }
    false
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/es2021.string-replaceAll.ts")
  }
}
