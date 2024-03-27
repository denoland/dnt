// Copyright 2018-2024 the Deno authors. MIT license.

use deno_ast::view::Node;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct PromiseWithResolversPolyfill;

impl Polyfill for PromiseWithResolversPolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    // (target as u32) < (ScriptTarget::ES2021 as u32)
    true // just always use it for the time being
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    context.has_global_property_access(node, "Promise", "withResolvers")
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/es2021.promise-withResolvers.ts")
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::polyfills::PolyfillTester;

  #[test]
  pub fn finds_when_matches() {
    let tester =
      PolyfillTester::new(Box::new(|| Box::new(PromiseWithResolversPolyfill)));
    assert_eq!(tester.matches("Promise.withResolvers"), true);
    assert_eq!(
      tester.matches("class Promise {} Promise.withResolvers"),
      false
    );
    assert_eq!(tester.matches("Other.withResolvers"), false);
    assert_eq!(tester.matches("Promise.withResolvers2"), false);
    assert_eq!(tester.matches("const { withResolvers } = Promise;"), true);
    assert_eq!(
      tester.matches("const { withResolvers: test } = Promise;"),
      true
    );
    assert_eq!(
      tester.matches("const { \"withResolvers\": test } = Promise;"),
      true
    );
    assert_eq!(tester.matches("const { withResolvers } = other;"), false);
    assert_eq!(
      tester.matches("class Promise {} const { withResolvers } = Promise;"),
      false
    );
    assert_eq!(tester.matches("const { ...rest } = Promise;"), true); // unknown, so true
    assert_eq!(
      tester.matches("const { [computed]: test } = Promise;"),
      true
    ); // unknown, so true
  }
}
