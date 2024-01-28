// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use deno_ast::view::*;
use deno_ast::ModuleSpecifier;
use deno_ast::SourcePos;
use deno_ast::SourceRange;
use deno_ast::SourceRanged;
use deno_ast::SourceRangedForSpanned;
use deno_ast::SourceTextInfoProvider;
use deno_ast::TextChange;

use crate::graph::ModuleGraph;
use crate::mappings::Mappings;
use crate::utils::get_relative_specifier;

pub struct GetImportExportsTextChangesParams<'a> {
  pub specifier: &'a ModuleSpecifier,
  pub module_graph: &'a ModuleGraph,
  pub mappings: &'a Mappings,
  pub program: Program<'a>,
  pub package_specifier_mappings: &'a HashMap<ModuleSpecifier, String>,
}

struct Context<'a> {
  program: Program<'a>,
  specifier: &'a ModuleSpecifier,
  module_graph: &'a ModuleGraph,
  mappings: &'a Mappings,
  output_file_path: &'a PathBuf,
  text_changes: Vec<TextChange>,
  package_specifier_mappings: &'a HashMap<ModuleSpecifier, String>,
}

pub fn get_import_exports_text_changes(
  params: &GetImportExportsTextChangesParams<'_>,
) -> Result<Vec<TextChange>> {
  let mut context = Context {
    program: params.program,
    specifier: params.specifier,
    module_graph: params.module_graph,
    mappings: params.mappings,
    output_file_path: params.mappings.get_file_path(params.specifier),
    text_changes: Vec::new(),
    package_specifier_mappings: params.package_specifier_mappings,
  };

  visit_children(params.program.as_node(), &mut context)?;

  Ok(context.text_changes)
}

fn visit_children(node: Node, context: &mut Context) -> Result<()> {
  for child in node.children() {
    match child {
      Node::ImportDecl(import_decl) => {
        visit_module_specifier(import_decl.src, context);
        if let Some(asserts) = import_decl.with {
          visit_import_attributes(asserts, context);
        }
      }
      Node::ExportAll(export_all) => {
        visit_module_specifier(export_all.src, context);
        if let Some(asserts) = export_all.with {
          visit_import_attributes(asserts, context);
        }
      }
      Node::NamedExport(named_export) => {
        if let Some(src) = &named_export.src {
          visit_module_specifier(src, context);
        }
        if let Some(asserts) = named_export.with {
          visit_import_attributes(asserts, context);
        }
      }
      Node::TsImportType(ts_import_type) => {
        visit_module_specifier(ts_import_type.arg, context);
      }
      Node::TsModuleDecl(module_decl) => {
        if let TsModuleName::Str(src) = &module_decl.id {
          visit_module_specifier(src, context);
        }
      }
      Node::CallExpr(call_expr) => {
        if matches!(call_expr.callee, Callee::Import(_)) {
          if let Some(Node::Str(src)) =
            call_expr.args.first().map(|a| a.expr.as_node())
          {
            visit_module_specifier(src, context);
            if call_expr.args.len() > 1 {
              let assert_arg = call_expr.args[1];
              let comma_token =
                assert_arg.previous_token_fast(context.program).unwrap();
              context.text_changes.push(TextChange {
                range: create_range(
                  comma_token.start(),
                  assert_arg.end(),
                  context,
                ),
                new_text: String::new(),
              });
            }
          }
        } else {
          visit_children(child, context)?;
        }
      }
      _ => {
        visit_children(child, context)?;
      }
    }
  }

  Ok(())
}

fn visit_module_specifier(str: &Str, context: &mut Context) {
  let value = str.value().to_string();
  let specifier = context
    .module_graph
    .resolve_dependency(&value, context.specifier);
  let specifier = match specifier {
    Some(s) => s,
    None => return,
  };

  let new_text = if let Some(bare_specifier) =
    context.package_specifier_mappings.get(&specifier)
  {
    bare_specifier.to_string()
  } else {
    let specifier_file_path = context.mappings.get_file_path(&specifier);
    get_relative_specifier(context.output_file_path, specifier_file_path)
  };

  context.text_changes.push(TextChange {
    range: create_range(str.start() + 1, str.end() - 1, context),
    new_text,
  });
}

fn visit_import_attributes(asserts: &ObjectLit, context: &mut Context) {
  let with_token = asserts.previous_token_fast(context.program).unwrap();
  debug_assert!(matches!(
    with_token.text_fast(context.program),
    "with" | "assert"
  ));
  let previous_token = with_token.previous_token_fast(context.program).unwrap();
  context.text_changes.push(TextChange {
    range: create_range(previous_token.end(), asserts.end(), context),
    new_text: String::new(),
  });
}

fn create_range(
  start: SourcePos,
  end: SourcePos,
  context: &Context,
) -> std::ops::Range<usize> {
  SourceRange::new(start, end)
    .as_byte_range(context.program.text_info().range().start)
}
