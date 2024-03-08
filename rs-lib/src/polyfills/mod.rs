// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use deno_ast::swc::common::SyntaxContext;
use deno_ast::view::Expr;
use deno_ast::view::Node;
use deno_ast::view::ObjectPatProp;
use deno_ast::view::Pat;
use deno_ast::view::Program;
use deno_ast::view::PropName;
use deno_ast::SourceRanged;

use crate::Dependency;
use crate::ScriptTarget;

mod array_find_last;
mod array_from_async;
mod error_cause;
mod import_meta;
mod object_has_own;
mod promise_with_resolvers;
mod string_replace_all;

pub trait Polyfill {
  fn use_for_target(&self, target: ScriptTarget) -> bool;
  fn visit_node(
    &self,
    node: Node,
    context: &PolyfillVisitContext<'_, '_>,
  ) -> bool;
  fn get_file_text(&self) -> &'static str;
  fn dependencies(&self) -> Vec<Dependency> {
    Vec::new()
  }
}

pub struct PolyfillVisitContext<'a, 'b> {
  pub program: Program<'b>,
  pub unresolved_context: SyntaxContext,
  pub top_level_decls: &'a HashSet<String>,
}

impl<'a, 'b> PolyfillVisitContext<'a, 'b> {
  pub fn has_global_property_access(
    &self,
    node: Node,
    global_name: &str,
    property_name: &str,
  ) -> bool {
    match node {
      // ex. Object.hasOwn
      Node::MemberExpr(member_expr) => {
        if let Expr::Ident(obj_ident) = &member_expr.obj {
          obj_ident.ctxt() == self.unresolved_context
            && !self.top_level_decls.contains(global_name)
            && obj_ident.text_fast(self.program) == global_name
            && member_expr.prop.text_fast(self.program) == property_name
        } else {
          false
        }
      }
      // ex. const { hasOwn } = Object;
      Node::VarDeclarator(decl) => {
        let init = match &decl.init {
          Some(Expr::Ident(ident)) => ident,
          _ => return false,
        };
        let props = match &decl.name {
          Pat::Object(obj) => &obj.props,
          _ => return false,
        };
        init.ctxt() == self.unresolved_context
          && !self.top_level_decls.contains(global_name)
          && init.text_fast(self.program) == global_name
          && props.iter().any(|prop| {
            match prop {
              ObjectPatProp::Rest(_) => true, // unknown, so include
              ObjectPatProp::Assign(assign) => {
                assign.key.text_fast(self.program) == property_name
              }
              ObjectPatProp::KeyValue(key_value) => match &key_value.key {
                PropName::BigInt(_) | PropName::Num(_) => false,
                PropName::Computed(_) => true, // unknown, so include
                PropName::Ident(ident) => {
                  ident.text_fast(self.program) == property_name
                }
                PropName::Str(str) => str.value() == property_name,
              },
            }
          })
      }
      _ => false,
    }
  }
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
    Box::new(array_from_async::ArrayFromAsyncPolyfill),
    Box::new(import_meta::ImportMetaPolyfill),
    Box::new(promise_with_resolvers::PromiseWithResolversPolyfill),
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
    use deno_graph::ParseOptions;

    use crate::analyze::get_top_level_decls;
    use crate::parser::ScopeAnalysisParser;
    use crate::visitors::fill_polyfills;
    use crate::visitors::FillPolyfillsParams;

    let parser = ScopeAnalysisParser::default();
    let parsed_source = parser
      .parse_module(ParseOptions {
        specifier: &ModuleSpecifier::parse("file://test.ts").unwrap(),
        source: text.into(),
        media_type: MediaType::TypeScript,
        scope_analysis: true,
      })
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
