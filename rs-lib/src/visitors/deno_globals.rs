// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::swc::common::SyntaxContext;
use deno_ast::swc::utils::ident::IdentLike;
use deno_ast::view::*;

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
    get_top_level_declarations(params.program, params.top_level_context);
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
      && !should_ignore(ident.into(), context)
      && !is_in_type(ident.into())
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

fn should_ignore(node: Node, context: &Context) -> bool {
  context
    .ignore_line_indexes
    .contains(&node.span().start_line_fast(context.program))
    || in_left_hand_assignment(node)
    || is_declaration_ident(node)
    || is_directly_in_condition(node)
}

fn is_directly_in_condition(node: Node) -> bool {
  match node.parent() {
    Some(Node::BinExpr(_)) => true,
    Some(Node::IfStmt(_)) => true,
    Some(Node::UnaryExpr(expr)) => expr.op() == UnaryOp::TypeOf,
    Some(Node::CondExpr(cond_expr)) => {
      cond_expr.test.span().contains(node.span())
    }
    _ => false,
  }
}

fn is_declaration_ident(node: Node) -> bool {
  if let Some(parent) = node.parent() {
    let span = match parent {
      Node::BindingIdent(decl) => Some(decl.id.span()),
      Node::ClassDecl(decl) => Some(decl.ident.span()),
      Node::ClassExpr(decl) => Some(decl.ident.span()),
      Node::TsInterfaceDecl(decl) => Some(decl.id.span()),
      Node::FnDecl(decl) => Some(decl.ident.span()),
      Node::FnExpr(decl) => Some(decl.ident.span()),
      Node::TsModuleDecl(decl) => Some(decl.id.span()),
      Node::TsNamespaceDecl(decl) => Some(decl.id.span()),
      Node::VarDeclarator(decl) => Some(decl.name.span()),
      Node::ImportNamedSpecifier(decl) => Some(decl.span()),
      Node::ExportNamedSpecifier(decl) => Some(decl.span()),
      Node::ExportNamespaceSpecifier(decl) => Some(decl.span()),
      Node::ImportDefaultSpecifier(decl) => Some(decl.span()),
      Node::ExportDefaultSpecifier(decl) => Some(decl.span()),
      Node::KeyValuePatProp(decl) => Some(decl.key.span()),
      Node::AssignPatProp(decl) => Some(decl.key.span()),
      _ => None,
    };
    match span {
      Some(span) => span.contains(node.span()),
      None => false,
    }
  } else {
    false
  }
}

fn in_left_hand_assignment(node: Node) -> bool {
  for ancestor in node.ancestors() {
    if let Node::AssignExpr(expr) = ancestor {
      return expr.left.span().contains(node.span());
    }
  }
  false
}

fn is_in_type(mut node: Node) -> bool {
  // todo: add unit tests and investigate if there's something in swc that does this?
  while let Some(parent) = node.parent() {
    let is_type = match parent {
      Node::ArrayLit(_)
      | Node::ArrayPat(_)
      | Node::ArrowExpr(_)
      | Node::AssignExpr(_)
      | Node::AssignPat(_)
      | Node::AssignPatProp(_)
      | Node::AssignProp(_)
      | Node::AwaitExpr(_)
      | Node::BinExpr(_)
      | Node::BindingIdent(_)
      | Node::BlockStmt(_)
      | Node::BreakStmt(_)
      | Node::CallExpr(_)
      | Node::CatchClause(_)
      | Node::Class(_)
      | Node::ClassDecl(_)
      | Node::ClassExpr(_)
      | Node::ClassMethod(_)
      | Node::ClassProp(_)
      | Node::ComputedPropName(_)
      | Node::CondExpr(_)
      | Node::Constructor(_)
      | Node::ContinueStmt(_)
      | Node::DebuggerStmt(_)
      | Node::Decorator(_)
      | Node::DoWhileStmt(_)
      | Node::EmptyStmt(_)
      | Node::ExportAll(_)
      | Node::ExportDecl(_)
      | Node::ExportDefaultDecl(_)
      | Node::ExportDefaultExpr(_)
      | Node::ExportDefaultSpecifier(_)
      | Node::ExportNamedSpecifier(_)
      | Node::ExportNamespaceSpecifier(_)
      | Node::ExprOrSpread(_)
      | Node::ExprStmt(_)
      | Node::FnDecl(_)
      | Node::FnExpr(_)
      | Node::ForInStmt(_)
      | Node::ForOfStmt(_)
      | Node::ForStmt(_)
      | Node::Function(_)
      | Node::GetterProp(_)
      | Node::IfStmt(_)
      | Node::ImportDecl(_)
      | Node::ImportDefaultSpecifier(_)
      | Node::ImportNamedSpecifier(_)
      | Node::ImportStarAsSpecifier(_)
      | Node::Invalid(_)
      | Node::UnaryExpr(_)
      | Node::UpdateExpr(_)
      | Node::VarDecl(_)
      | Node::VarDeclarator(_)
      | Node::WhileStmt(_)
      | Node::WithStmt(_)
      | Node::YieldExpr(_)
      | Node::JSXAttr(_)
      | Node::JSXClosingElement(_)
      | Node::JSXClosingFragment(_)
      | Node::JSXElement(_)
      | Node::JSXEmptyExpr(_)
      | Node::JSXExprContainer(_)
      | Node::JSXFragment(_)
      | Node::JSXMemberExpr(_)
      | Node::JSXNamespacedName(_)
      | Node::JSXOpeningElement(_)
      | Node::JSXOpeningFragment(_)
      | Node::JSXSpreadChild(_)
      | Node::JSXText(_)
      | Node::KeyValuePatProp(_)
      | Node::KeyValueProp(_)
      | Node::LabeledStmt(_)
      | Node::MetaPropExpr(_)
      | Node::MethodProp(_)
      | Node::Module(_)
      | Node::NamedExport(_)
      | Node::NewExpr(_)
      | Node::ObjectLit(_)
      | Node::ObjectPat(_)
      | Node::OptChainExpr(_)
      | Node::Param(_)
      | Node::ParenExpr(_)
      | Node::PrivateMethod(_)
      | Node::PrivateName(_)
      | Node::PrivateProp(_)
      | Node::Regex(_)
      | Node::RestPat(_)
      | Node::ReturnStmt(_)
      | Node::Script(_)
      | Node::SeqExpr(_)
      | Node::SetterProp(_)
      | Node::SpreadElement(_)
      | Node::StaticBlock(_)
      | Node::Super(_)
      | Node::SwitchCase(_)
      | Node::SwitchStmt(_)
      | Node::TaggedTpl(_)
      | Node::ThisExpr(_)
      | Node::ThrowStmt(_)
      | Node::Tpl(_)
      | Node::TplElement(_)
      | Node::TryStmt(_)
      | Node::TsEnumDecl(_)
      | Node::TsEnumMember(_)
      | Node::TsExportAssignment(_)
      | Node::TsExternalModuleRef(_)
      | Node::TsImportEqualsDecl(_)
      | Node::TsModuleBlock(_)
      | Node::TsModuleDecl(_)
      | Node::TsNamespaceDecl(_)
      | Node::TsNamespaceExportDecl(_) => Some(false),

      Node::TsArrayType(_)
      | Node::TsCallSignatureDecl(_)
      | Node::TsConditionalType(_)
      | Node::TsConstAssertion(_)
      | Node::TsConstructSignatureDecl(_)
      | Node::TsConstructorType(_)
      | Node::TsExprWithTypeArgs(_)
      | Node::TsFnType(_)
      | Node::TsGetterSignature(_)
      | Node::TsImportType(_)
      | Node::TsIndexSignature(_)
      | Node::TsIndexedAccessType(_)
      | Node::TsInferType(_)
      | Node::TsInterfaceBody(_)
      | Node::TsInterfaceDecl(_)
      | Node::TsIntersectionType(_)
      | Node::TsKeywordType(_)
      | Node::TsLitType(_)
      | Node::TsMappedType(_)
      | Node::TsMethodSignature(_)
      | Node::TsNonNullExpr(_)
      | Node::TsOptionalType(_)
      | Node::TsParamProp(_)
      | Node::TsParenthesizedType(_)
      | Node::TsPropertySignature(_)
      | Node::TsQualifiedName(_)
      | Node::TsRestType(_)
      | Node::TsSetterSignature(_)
      | Node::TsThisType(_)
      | Node::TsTplLitType(_)
      | Node::TsTupleElement(_)
      | Node::TsTupleType(_)
      | Node::TsTypeAliasDecl(_)
      | Node::TsTypeAnn(_)
      | Node::TsTypeLit(_)
      | Node::TsTypeOperator(_)
      | Node::TsTypeParam(_)
      | Node::TsTypeParamDecl(_)
      | Node::TsTypeParamInstantiation(_)
      | Node::TsTypePredicate(_)
      | Node::TsTypeQuery(_)
      | Node::TsTypeRef(_)
      | Node::TsUnionType(_) => Some(true),

      // may be a type
      Node::TsTypeAssertion(expr) => {
        Some(expr.type_ann.span().contains(node.span()))
      }
      Node::TsAsExpr(expr) => Some(expr.type_ann.span().contains(node.span())),

      // still need more info
      Node::BigInt(_)
      | Node::Bool(_)
      | Node::Null(_)
      | Node::Number(_)
      | Node::MemberExpr(_)
      | Node::Str(_)
      | Node::Ident(_) => None,
    };
    if let Some(is_type) = is_type {
      return is_type;
    }
    node = parent;
  }

  false
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

fn get_top_level_declarations(
  program: &Program,
  top_level_context: SyntaxContext,
) -> HashSet<String> {
  use deno_ast::swc::common::collections::AHashSet;
  use deno_ast::swc::utils::collect_decls_with_ctxt;
  use deno_ast::swc::utils::Id;

  let results: AHashSet<Id> = match program {
    Program::Module(module) => {
      collect_decls_with_ctxt(module.inner, top_level_context)
    }
    Program::Script(script) => {
      collect_decls_with_ctxt(script.inner, top_level_context)
    }
  };
  results.iter().map(|v| v.0.to_string()).collect()
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
