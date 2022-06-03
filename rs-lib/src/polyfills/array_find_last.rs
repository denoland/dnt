// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_ast::view::Callee;
use deno_ast::view::Expr;
use deno_ast::view::Node;
use deno_ast::SourceRanged;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct ArrayFindLastPolyfill;

impl Polyfill for ArrayFindLastPolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    true
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    if let Node::CallExpr(expr) = node {
      if let Callee::Expr(Expr::Member(callee)) = expr.callee {
        if matches!(expr.args.len(), 1 | 2)
          && matches!(
            callee.prop.text_fast(context.program),
            "findLast" | "findLastIndex"
          )
        {
          return true;
        }
      }
    }
    false
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/esnext.array-findLast.ts")
  }
}
