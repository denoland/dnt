// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use deno_ast::swc::common::SyntaxContext;
use deno_ast::view::Node;
use deno_ast::view::Program;

use crate::ScriptTarget;

mod array_find_last;
mod error_cause;
mod object_has_own;
mod string_replace_all;
mod import_meta;

pub trait Polyfill {
  fn use_for_target(&self, target: ScriptTarget) -> bool;
  fn visit_node(
    &self,
    node: Node,
    context: &PolyfillVisitContext<'_, '_>,
  ) -> bool;
  fn get_file_text(&self) -> &'static str;
}

pub struct PolyfillVisitContext<'a, 'b> {
  pub program: Program<'b>,
  pub unresolved_context: SyntaxContext,
  pub top_level_decls: &'a HashSet<String>,
}

pub fn polyfills_for_target(target: ScriptTarget) -> Vec<Box<dyn Polyfill>> {
  all_polyfills()
    .into_iter()
    .filter(|p| p.use_for_target(target))
    .collect()
}

fn all_polyfills() -> Vec<Box<dyn Polyfill>> {
  vec![
    Box::new(object_has_own::ObjectHasOwnPolyfill),
    Box::new(error_cause::ErrorCausePolyfill),
    Box::new(string_replace_all::StringReplaceAllPolyfill),
    Box::new(array_find_last::ArrayFindLastPolyfill),
    Box::new(import_meta::ImportMetaPolyfill),
  ]
}

pub fn build_polyfill_file(polyfills: &[Box<dyn Polyfill>]) -> Option<String> {
  if polyfills.is_empty() {
    return None;
  }

  let mut file_text = String::new();

  for polyfill in polyfills {
    file_text.push_str(polyfill.get_file_text());
  }

  Some(file_text)
}

#[cfg(test)]
struct PolyfillTester {
  create_polyfill: Box<dyn Fn() -> Box<dyn Polyfill>>,
}

#[cfg(test)]
impl PolyfillTester {
  pub fn new(create_polyfill: Box<dyn Fn() -> Box<dyn Polyfill>>) -> Self {
    Self { create_polyfill }
  }

  pub fn matches(&self, text: &str) -> bool {
    use deno_ast::MediaType;
    use deno_ast::ModuleSpecifier;
    use deno_graph::ModuleParser;

    use crate::analyze::get_top_level_decls;
    use crate::parser::ScopeAnalysisParser;
    use crate::visitors::fill_polyfills;
    use crate::visitors::FillPolyfillsParams;

    let parser = ScopeAnalysisParser::new();
    let parsed_source = parser
      .parse_module(
        &ModuleSpecifier::parse("file://test.ts").unwrap(),
        text.into(),
        MediaType::TypeScript,
      )
      .unwrap();
    parsed_source.with_view(|program| {
      let mut searching_polyfills = vec![(self.create_polyfill)()];
      let mut found_polyfills = Vec::new();
      let unresolved_context = parsed_source.unresolved_context();
      let top_level_decls = get_top_level_decls(program, unresolved_context);
      fill_polyfills(&mut FillPolyfillsParams {
        program,
        unresolved_context,
        top_level_decls: &top_level_decls,
        searching_polyfills: &mut searching_polyfills,
        found_polyfills: &mut found_polyfills,
      });
      !found_polyfills.is_empty()
    })
  }
}
