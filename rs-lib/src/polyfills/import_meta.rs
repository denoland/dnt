// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use deno_ast::view::Expr;
use deno_ast::view::Node;
use deno_ast::SourceRanged;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct ImportMetaPolyfill;

impl Polyfill for ImportMetaPolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    true
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    if let Node::MemberExpr(expr) = node {
      if let Expr::MetaProp(_meta) = expr.obj {
        let text = expr.prop.text_fast(context.program);
        if text == "main" || text == "resolve" {
          return true;
        }
      }
    }
    false
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/deno.import-meta.ts")
  }
}
