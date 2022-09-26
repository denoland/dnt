// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use deno_ast::swc::common::SyntaxContext;
use deno_ast::view::*;
use deno_ast::SourceRanged;

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
      Node::BindingIdent(decl) => decl.id.range().contains(&node.range()),
      Node::ClassDecl(decl) => decl.ident.range().contains(&node.range()),
      Node::ClassExpr(decl) => decl
        .ident
        .as_ref()
        .map(|i| i.range().contains(&node.range()))
        .unwrap_or(false),
      Node::TsInterfaceDecl(decl) => decl.id.range().contains(&node.range()),
      Node::FnDecl(decl) => decl.ident.range().contains(&node.range()),
      Node::FnExpr(decl) => decl
        .ident
        .as_ref()
        .map(|i| i.range().contains(&node.range()))
        .unwrap_or(false),
      Node::TsModuleDecl(decl) => decl.id.range().contains(&node.range()),
      Node::TsNamespaceDecl(decl) => decl.id.range().contains(&node.range()),
      Node::VarDeclarator(decl) => decl.name.range().contains(&node.range()),
      Node::ImportNamedSpecifier(decl) => {
        decl.local.range().contains(&node.range())
      }
      Node::ImportDefaultSpecifier(decl) => {
        decl.local.range().contains(&node.range())
      }
      Node::ImportStarAsSpecifier(decl) => decl.range().contains(&node.range()),
      Node::KeyValuePatProp(decl) => decl.key.range().contains(&node.range()),
      Node::AssignPatProp(decl) => decl.key.range().contains(&node.range()),
      _ => false,
    }
  } else {
    false
  }
}
