// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_ast::swc::common::Spanned;
use deno_ast::view::Node;
use deno_ast::view::NodeTrait;
use deno_ast::view::SpannedExt;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct ObjectHasOwnPolyfill;

impl Polyfill for ObjectHasOwnPolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    true
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    if let Node::CallExpr(expr) = node {
      if let Node::MemberExpr(callee) = expr.callee.as_node() {
        if callee.text_fast(context.program) == "Object.hasOwn"
          && callee.obj.span().ctxt == context.top_level_context
        {
          return true;
        }
      }
    }
    false
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/esnext.object-has-own.ts")
  }
}
