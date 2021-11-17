// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::swc::common::SyntaxContext;
use deno_ast::swc::utils::ident::IdentLike;
use deno_ast::view::*;

use super::analyze::get_top_level_decls;
use super::analyze::is_directly_in_condition;
use super::analyze::is_in_left_hand_assignment;
use super::analyze::is_in_type;
use crate::text_changes::TextChange;

const DENO_SHIM_GLOBAL_NAMES: [&str; 14] = [
  "Blob",
  "crypto",
  "Deno",
  "fetch",
  "File",
  "FormData",
  "Headers",
  "Request",
  "Response",
  "alert",
  "confirm",
  "prompt",
  "setTimeout",
  "setInterval",
];

pub struct GetDenoGlobalTextChangesParams<'a> {
  pub program: &'a Program<'a>,
  pub top_level_context: SyntaxContext,
  pub shim_package_name: &'a str,
}

struct Context<'a> {
  program: &'a Program<'a>,
  top_level_context: SyntaxContext,
  top_level_decls: &'a HashSet<String>,
  import_shim: bool,
  text_changes: Vec<TextChange>,
  ignore_line_indexes: HashSet<usize>,
}

pub fn get_deno_global_text_changes(
  params: &GetDenoGlobalTextChangesParams<'_>,
) -> Vec<TextChange> {
  let top_level_decls =
    get_top_level_decls(params.program, params.top_level_context);
  let ignore_line_indexes = get_ignore_line_indexes(params.program);
  let mut context = Context {
    program: params.program,
    top_level_context: params.top_level_context,
    top_level_decls: &top_level_decls,
    import_shim: false,
    text_changes: Vec::new(),
    ignore_line_indexes,
  };
  let program = params.program;

  // currently very crude. This should be improved to only look
  // at binding declarations
  let all_ident_names = get_all_ident_names(context.program);
  let deno_shim_name = get_unique_name("denoShim", &all_ident_names);

  visit_children(program.into(), &deno_shim_name, &mut context);

  if context.import_shim {
    context.text_changes.push(TextChange {
      span: Span::new(BytePos(0), BytePos(0), Default::default()),
      new_text: format!(
        "import * as {} from \"{}\";\n",
        deno_shim_name, params.shim_package_name,
      ),
    });
  }

  context.text_changes
}

fn visit_children(node: Node, import_name: &str, context: &mut Context) {
  for child in node.children() {
    visit_children(child, import_name, context);
  }

  if let Node::Ident(ident) = node {
    let id = ident.inner.to_id();
    let is_top_level_context = id.1 == context.top_level_context;
    let ident_text = ident.text_fast(context.program);
    if is_top_level_context
      && ident_text == "globalThis"
      && !should_ignore_global_this(ident, context)
    {
      context.text_changes.push(TextChange {
        span: ident.span(),
        new_text: format!("({{ ...{}, ...globalThis }})", import_name),
      });
      context.import_shim = true;
    }

    // check if Deno should be imported
    if is_top_level_context {
      for name in DENO_SHIM_GLOBAL_NAMES {
        if ident_text == name
          && !context.top_level_decls.contains(name)
          && !should_ignore(ident.into(), context)
        {
          context.text_changes.push(TextChange {
            span: ident.span(),
            new_text: format!("{}.{}", import_name, ident_text),
          });
          context.import_shim = true;
        }
      }
    }
  }
}

fn should_ignore_global_this(ident: &Ident, context: &Context) -> bool {
  if should_ignore(ident.into(), context) || is_in_type(ident.into()) {
    return true;
  }

  // don't inject the Deno namespace when it's a member expression
  // not like `globalThis.Deno`
  if let Some(parent_member_expr) = ident.parent().to::<MemberExpr>() {
    if parent_member_expr.obj.span().contains(ident.span()) {
      match parent_member_expr.prop.into() {
        Node::Ident(prop_ident) => {
          if prop_ident.sym().as_ref() != "Deno" {
            return true;
          }
        }
        Node::Str(str) => {
          if str.value().as_ref() != "Deno" {
            return true;
          }
        }
        _ => {}
      }
    }
  }

  false
}

fn should_ignore(node: Node, context: &Context) -> bool {
  context
    .ignore_line_indexes
    .contains(&node.span().start_line_fast(context.program))
    || is_in_left_hand_assignment(node)
    || is_declaration_ident(node)
    || is_directly_in_condition(node)
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

fn get_ignore_line_indexes(program: &Program) -> HashSet<usize> {
  let mut result = HashSet::new();
  for comment in program.comment_container().unwrap().all_comments() {
    if comment
      .text
      .trim()
      .to_lowercase()
      .starts_with("deno-shim-ignore")
    {
      if let Some(next_token) = comment.next_token_fast(program) {
        result.insert(next_token.span.lo.start_line_fast(program));
      }
    }
  }
  result
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
