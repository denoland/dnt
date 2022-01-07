// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_ast::view::Node;
use deno_ast::view::SpannedExt;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct WeakRefPolyfill;

impl Polyfill for WeakRefPolyfill {
  fn use_for_target(&self, target: ScriptTarget) -> bool {
    (target as u32) < (ScriptTarget::ES2021 as u32)
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    if let Node::Ident(ident) = node {
      if ident.inner.span.ctxt == context.top_level_context
        && ident.text_fast(context.program) == "WeakRef"
      {
        return true;
      }
    }
    false
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/es2021.weak-ref.ts")
  }
}
