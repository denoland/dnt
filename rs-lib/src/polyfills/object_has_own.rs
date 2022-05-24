// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_ast::view::Expr;
use deno_ast::view::Node;
use deno_ast::view::ObjectPatProp;
use deno_ast::view::Pat;
use deno_ast::view::PropName;
use deno_ast::SourceRanged;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct ObjectHasOwnPolyfill;

impl Polyfill for ObjectHasOwnPolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    true
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    match node {
      // Object.hasOwn
      Node::MemberExpr(member_expr) => {
        if let Expr::Ident(obj_ident) = &member_expr.obj {
          obj_ident.ctxt() == context.unresolved_context
            && !context.top_level_decls.contains("Object")
            && obj_ident.text_fast(context.program) == "Object"
            && member_expr.prop.text_fast(context.program) == "hasOwn"
        } else {
          false
        }
      }
      // const { hasOwn } = Object;
      Node::VarDeclarator(decl) => {
        let init = match &decl.init {
          Some(Expr::Ident(ident)) => ident,
          _ => return false,
        };
        let props = match &decl.name {
          Pat::Object(obj) => &obj.props,
          _ => return false,
        };
        init.ctxt() == context.unresolved_context
          && !context.top_level_decls.contains("Object")
          && init.text_fast(context.program) == "Object"
          && props.iter().any(|prop| {
            match prop {
              ObjectPatProp::Rest(_) => true, // unknown, so include
              ObjectPatProp::Assign(assign) => {
                assign.key.text_fast(context.program) == "hasOwn"
              }
              ObjectPatProp::KeyValue(key_value) => match &key_value.key {
                PropName::BigInt(_) | PropName::Num(_) => false,
                PropName::Computed(_) => true, // unknown, so include
                PropName::Ident(ident) => {
                  ident.text_fast(context.program) == "hasOwn"
                }
                PropName::Str(str) => str.value() == "hasOwn",
              },
            }
          })
      }
      _ => false,
    }
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/esnext.object-has-own.ts")
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::polyfills::PolyfillTester;

  #[test]
  pub fn finds_when_matches() {
    let tester =
      PolyfillTester::new(Box::new(|| Box::new(ObjectHasOwnPolyfill)));
    assert_eq!(tester.matches("Object.hasOwn"), true);
    assert_eq!(tester.matches("class Object {} Object.hasOwn"), false);
    assert_eq!(tester.matches("Other.hasOwn"), false);
    assert_eq!(tester.matches("Object.hasOther"), false);
    assert_eq!(tester.matches("const { hasOwn } = Object;"), true);
    assert_eq!(tester.matches("const { hasOwn: test } = Object;"), true);
    assert_eq!(tester.matches("const { \"hasOwn\": test } = Object;"), true);
    assert_eq!(tester.matches("const { hasOwn } = other;"), false);
    assert_eq!(
      tester.matches("class Object {} const { hasOwn } = Object;"),
      false
    );
    assert_eq!(tester.matches("const { ...rest } = Object;"), true); // unknown, so true
    assert_eq!(tester.matches("const { [computed]: test } = Object;"), true); // unknown, so true
  }
}
