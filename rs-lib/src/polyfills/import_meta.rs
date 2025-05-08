// Copyright 2018-2024 the Deno authors. MIT license.

use deno_ast::view::Expr;
use deno_ast::view::Node;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct ImportMetaPolyfill;

impl Polyfill for ImportMetaPolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    true
  }

  fn visit_node(&self, node: Node, _context: &PolyfillVisitContext) -> bool {
    if let Node::MemberExpr(expr) = node {
      if let Expr::MetaProp(_meta) = expr.obj {
        return true;
      }
    }
    false
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/deno.import-meta.ts")
  }
}
