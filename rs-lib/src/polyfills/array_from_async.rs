// Copyright 2018-2024 the Deno authors. MIT license.

use deno_ast::view::Node;

use super::Polyfill;
use super::PolyfillVisitContext;
use crate::ScriptTarget;

pub struct ArrayFromAsyncPolyfill;

impl Polyfill for ArrayFromAsyncPolyfill {
  fn use_for_target(&self, _target: ScriptTarget) -> bool {
    true
  }

  fn visit_node(&self, node: Node, context: &PolyfillVisitContext) -> bool {
    context.has_global_property_access(node, "Array", "fromAsync")
  }

  fn get_file_text(&self) -> &'static str {
    include_str!("./scripts/esnext.array-fromAsync.ts")
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::polyfills::PolyfillTester;

  #[test]
  pub fn finds_when_matches() {
    let tester =
      PolyfillTester::new(Box::new(|| Box::new(ArrayFromAsyncPolyfill)));
    assert_eq!(tester.matches("Array.fromAsync"), true);
    assert_eq!(tester.matches("class Array {} Array.fromAsync"), false);
    assert_eq!(tester.matches("Other.fromAsync"), false);
    assert_eq!(tester.matches("Array.hasOther"), false);
    assert_eq!(tester.matches("const { fromAsync } = Array;"), true);
    assert_eq!(tester.matches("const { fromAsync: test } = Array;"), true);
    assert_eq!(
      tester.matches("const { \"fromAsync\": test } = Array;"),
      true
    );
    assert_eq!(tester.matches("const { fromAsync } = other;"), false);
    assert_eq!(
      tester.matches("class Array {} const { fromAsync } = Array;"),
      false
    );
    assert_eq!(tester.matches("const { ...rest } = Array;"), true); // unknown, so true
    assert_eq!(tester.matches("const { [computed]: test } = Array;"), true); // unknown, so true
  }
}
