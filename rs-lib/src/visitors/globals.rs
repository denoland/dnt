// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use deno_ast::swc::common::Spanned;
use deno_ast::swc::common::SyntaxContext;
use deno_ast::swc::utils::ident::IdentLike;
use deno_ast::view::*;
use deno_ast::TextChange;

use crate::analyze::is_in_type;
use crate::utils::text_change_for_prepend_statement_to_text;

pub struct GetGlobalTextChangesParams<'a> {
  pub program: &'a Program<'a>,
  pub top_level_context: SyntaxContext,
  pub shim_specifier: &'a str,
  pub shim_global_names: &'a HashSet<&'a str>,
  pub ignore_line_indexes: &'a HashSet<usize>,
  pub top_level_decls: &'a HashSet<String>,
}

pub struct GetGlobalTextChangesResult {
  pub text_changes: Vec<TextChange>,
  pub imported_shim: bool,
}

struct Context<'a> {
  program: &'a Program<'a>,
  top_level_context: SyntaxContext,
  top_level_decls: &'a HashSet<String>,
  shim_global_names: &'a HashSet<&'a str>,
  import_shim: bool,
  text_changes: Vec<TextChange>,
  ignore_line_indexes: &'a HashSet<usize>,
}

pub fn get_global_text_changes(
  params: &GetGlobalTextChangesParams<'_>,
) -> GetGlobalTextChangesResult {
  let mut context = Context {
    program: params.program,
    top_level_context: params.top_level_context,
    top_level_decls: params.top_level_decls,
    shim_global_names: params.shim_global_names,
    import_shim: false,
    text_changes: Vec::new(),
    ignore_line_indexes: params.ignore_line_indexes,
  };
  let program = params.program;

  // currently very crude. This should be improved to only look
  // at binding declarations
  let all_ident_names = get_all_ident_names(context.program);
  let global_shim_name = get_unique_name("dntShim", &all_ident_names);

  visit_children(program.into(), &global_shim_name, &mut context);

  if context.import_shim {
    context
      .text_changes
      .push(text_change_for_prepend_statement_to_text(
        program,
        &format!(
          "import * as {} from \"{}\";",
          global_shim_name, params.shim_specifier,
        ),
      ));
  }

  GetGlobalTextChangesResult {
    text_changes: context.text_changes,
    imported_shim: context.import_shim,
  }
}

fn visit_children(node: Node, import_name: &str, context: &mut Context) {
  for child in node.children() {
    visit_children(child, import_name, context);
  }

  if let Node::Ident(ident) = node {
    let id = ident.inner.to_id();
    let is_top_level_context = id.1 == context.top_level_context;
    let ident_text = ident.text_fast(context.program);

    if is_top_level_context {
      // change `window` -> `globalThis`
      if ident_text == "window" {
        if !context.top_level_decls.contains("window")
          && !has_ignore_comment(ident.into(), context)
        {
          if let Some(text_change) =
            get_global_this_text_change(ident, import_name, context)
          {
            context.text_changes.push(text_change);
            context.import_shim = true;
          } else {
            context.text_changes.push(TextChange::from_span_and_text(
              ident.span(),
              "globalThis".to_string(),
            ));
          }
        }
        return;
      }

      // check to replace globalThis
      if ident_text == "globalThis" {
        if let Some(text_change) =
          get_global_this_text_change(ident, import_name, context)
        {
          context.text_changes.push(text_change);
          context.import_shim = true;
        }
        return;
      }

      // check if global should be imported
      for &name in context.shim_global_names.iter() {
        if ident_text == name
          && !context.top_level_decls.contains(name)
          && !should_ignore(ident.into(), context)
        {
          context.text_changes.push(TextChange::from_span_and_text(
            ident.span(),
            format!("{}.{}", import_name, ident_text),
          ));
          context.import_shim = true;
          return;
        }
      }
    }
  }
}

fn get_global_this_text_change(
  ident: &Ident,
  import_name: &str,
  context: &Context,
) -> Option<TextChange> {
  if should_ignore_global_this(ident, context) {
    return None;
  }
  if is_in_type(ident.into()) {
    match ident.parent() {
      Node::TsQualifiedName(parent) => {
        let right_name = parent.right.text_fast(context.program);
        if context.shim_global_names.contains(&right_name) {
          Some(TextChange::from_span_and_text(
            parent.span(),
            format!(
              "{}.{}",
              import_name,
              // doesn't seem exactly right... will wait for a bug to open
              parent.right.text_fast(context.program),
            ),
          ))
        } else {
          None
        }
      }
      _ => None,
    }
  } else {
    Some(TextChange::from_span_and_text(
      ident.span(),
      format!("{}.dntGlobalThis", import_name),
    ))
  }
}

fn should_ignore_global_this(ident: &Ident, context: &Context) -> bool {
  if has_ignore_comment(ident.into(), context)
    || is_declaration_ident(ident.into())
  {
    return true;
  }

  // don't inject the globals when it's a member expression
  // not like `globalThis.<globalName>`
  if let Some(parent_member_expr) = ident.parent().to::<MemberExpr>() {
    if parent_member_expr.obj.span().contains(ident.span()) {
      match parent_member_expr.prop {
        MemberProp::Ident(prop_ident) => {
          if !context
            .shim_global_names
            .contains(prop_ident.sym().as_ref())
          {
            return true;
          }
        }
        MemberProp::Computed(computed) => {
          if let Expr::Lit(Lit::Str(str)) = computed.expr {
            if !context.shim_global_names.contains(str.value().as_ref()) {
              return true;
            }
          }
        }
        MemberProp::PrivateName(_) => {}
      }
    }
  }

  false
}

fn should_ignore(node: Node, context: &Context) -> bool {
  has_ignore_comment(node, context) || is_declaration_ident(node)
}

fn has_ignore_comment(node: Node, context: &Context) -> bool {
  context
    .ignore_line_indexes
    .contains(&node.span().start_line_fast(context.program))
}

fn is_declaration_ident(node: Node) -> bool {
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
      Node::ImportNamedSpecifier(decl) => decl.span().contains(node.span()),
      Node::ExportNamedSpecifier(decl) => decl.span().contains(node.span()),
      Node::ImportDefaultSpecifier(decl) => decl.span().contains(node.span()),
      Node::ExportDefaultSpecifier(decl) => decl.span().contains(node.span()),
      Node::ImportStarAsSpecifier(decl) => decl.span().contains(node.span()),
      Node::ExportNamespaceSpecifier(decl) => decl.span().contains(node.span()),
      Node::KeyValuePatProp(decl) => decl.key.span().contains(node.span()),
      Node::AssignPatProp(decl) => decl.key.span().contains(node.span()),
      _ => false,
    }
  } else {
    false
  }
}

fn get_all_ident_names(program: &Program) -> HashSet<String> {
  let mut result = HashSet::new();
  visit_children(&program.into(), &mut result);
  return result;

  fn visit_children(node: &Node, result: &mut HashSet<String>) {
    for child in node.children() {
      visit_children(&child, result);
    }

    if let Node::Ident(ident) = node {
      result.insert(ident.sym().to_string());
    }
  }
}

fn get_unique_name(name: &str, all_idents: &HashSet<String>) -> String {
  let mut count = 0;
  let mut new_name = name.to_string();
  while all_idents.contains(&new_name) {
    count += 1;
    new_name = format!("{}{}", name, count);
  }
  new_name
}
