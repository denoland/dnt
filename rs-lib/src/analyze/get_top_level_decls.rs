// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use deno_ast::swc::common::Spanned;
use deno_ast::swc::common::SyntaxContext;
use deno_ast::view::*;

pub fn get_top_level_decls(
  program: &Program,
  top_level_context: SyntaxContext,
) -> HashSet<String> {
  let mut results = HashSet::new();

  visit_children(program.into(), top_level_context, &mut results);

  results
}

fn visit_children(
  node: Node,
  top_level_context: SyntaxContext,
  results: &mut HashSet<String>,
) {
  if let Node::Ident(ident) = node {
    if ident.ctxt() == top_level_context && is_local_declaration_ident(node) {
      results.insert(ident.sym().to_string());
    }
  }

  for child in node.children() {
    visit_children(child, top_level_context, results);
  }
}

fn is_local_declaration_ident(node: Node) -> bool {
  if let Some(parent) = node.parent() {
    match parent {
      Node::BindingIdent(decl) => decl.id.span().contains(node.span()),
      Node::ClassDecl(decl) => decl.ident.span().contains(node.span()),
      Node::ClassExpr(decl) => decl.ident.span().contains(node.span()),
      Node::TsInterfaceDecl(decl) => decl.id.span().contains(node.span()),
      Node::FnDecl(decl) => decl.ident.span().contains(node.span()),
      Node::FnExpr(decl) => decl.ident.span().contains(node.span()),
      Node::TsModuleDecl(decl) => decl.id.span().contains(node.span()),
      Node::TsNamespaceDecl(decl) => decl.id.span().contains(node.span()),
      Node::VarDeclarator(decl) => decl.name.span().contains(node.span()),
      Node::ImportNamedSpecifier(decl) => {
        decl.local.span().contains(node.span())
      }
      Node::ImportDefaultSpecifier(decl) => {
        decl.local.span().contains(node.span())
      }
      Node::ImportStarAsSpecifier(decl) => decl.span().contains(node.span()),
      Node::KeyValuePatProp(decl) => decl.key.span().contains(node.span()),
      Node::AssignPatProp(decl) => decl.key.span().contains(node.span()),
      _ => false,
    }
  } else {
    false
  }
}
