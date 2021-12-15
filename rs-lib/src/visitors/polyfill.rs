// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use deno_ast::swc::common::Spanned;
use deno_ast::swc::common::SyntaxContext;
use deno_ast::view::*;

use crate::polyfills::Polyfill;

pub struct FillPolyfillsParams<'a> {
  pub program: &'a Program<'a>,
  pub top_level_context: SyntaxContext,
  pub polyfills: &'a mut HashSet<Polyfill>,
}

struct Context<'a> {
  program: &'a Program<'a>,
  top_level_context: SyntaxContext,
  polyfills: &'a mut HashSet<Polyfill>,
}

pub fn fill_polyfills(params: &mut FillPolyfillsParams<'_>) {
  let mut context = Context {
    program: params.program,
    top_level_context: params.top_level_context,
    polyfills: params.polyfills,
  };
  visit_children(context.program.as_node(), &mut context);
}

fn visit_children(node: Node, context: &mut Context) {
  for child in node.children() {
    visit_children(child, context);
  }

  match node {
    Node::CallExpr(expr) => {
      if let Node::MemberExpr(callee) = expr.callee.as_node() {
        if callee.text_fast(context.program) == "Object.hasOwn"
          && callee.obj.span().ctxt() == context.top_level_context
        {
          context.polyfills.insert(Polyfill::ObjectHasOwn);
        }
      }
    }
    Node::MemberExpr(expr) => {
      // very simple detection as we don't have type checking
      if expr.prop.text_fast(context.program) == "cause" {
        context.polyfills.insert(Polyfill::ErrorCause);
      }
    }
    _ => {}
  }
}
