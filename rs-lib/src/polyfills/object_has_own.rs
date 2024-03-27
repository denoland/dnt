// Copyright 2018-2024 the Deno authors. MIT license.

use deno_ast::view::Node;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct ObjectHasOwnPolyfill;

impl Polyfill for ObjectHasOwnPolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    true
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    context.has_global_property_access(node, "Object", "hasOwn")
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
