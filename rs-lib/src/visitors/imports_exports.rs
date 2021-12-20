// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;
use deno_ast::swc::common::Spanned;
use deno_ast::view::*;
use deno_ast::ModuleSpecifier;

use crate::graph::ModuleGraph;
use crate::mappings::Mappings;
use crate::text_changes::TextChange;
use crate::utils::get_relative_specifier;
use crate::utils::is_json_file_path;

pub struct GetImportExportsTextChangesParams<'a> {
  pub specifier: &'a ModuleSpecifier,
  pub module_graph: &'a ModuleGraph,
  pub mappings: &'a Mappings,
  pub program: &'a Program<'a>,
  pub specifier_mappings: &'a HashMap<ModuleSpecifier, String>,
}

struct Context<'a> {
  program: &'a Program<'a>,
  specifier: &'a ModuleSpecifier,
  module_graph: &'a ModuleGraph,
  mappings: &'a Mappings,
  output_file_path: &'a PathBuf,
  text_changes: Vec<TextChange>,
  specifier_mappings: &'a HashMap<ModuleSpecifier, String>,
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
    specifier_mappings: params.specifier_mappings,
  };

  visit_children(params.program.as_node(), &mut context)?;

  Ok(context.text_changes)
}

fn visit_children(node: Node, context: &mut Context) -> Result<()> {
  for child in node.children() {
    match child {
      Node::ImportDecl(import_decl) => {
        match analyze_specifier(import_decl.src, context) {
          Some(NewSpecifierOrJson::Specifier(new_text)) => {
            replace_specifier(import_decl.src, new_text, context)
          }
          Some(NewSpecifierOrJson::Json(json)) => {
            let default_import = import_decl
              .specifiers
              .iter()
              .filter_map(|s| {
                if let ImportSpecifier::Default(s) = s {
                  Some(s)
                } else {
                  None
                }
              })
              .next()
              .ok_or_else(|| {
                anyhow!(
                  "Could not find default specifier for import:\n  {}",
                  import_decl.text_fast(context.program)
                )
              })?;

            context.text_changes.push(TextChange {
              span: import_decl.span(),
              new_text: format!(
                "const {} = JSON.parse(`{}`);",
                default_import.text_fast(context.program),
                json.text.replace("`", "\\`")
              ),
            });
          }
          None => {}
        }
      }
      Node::ExportAll(export_all) => {
        match analyze_specifier(export_all.src, context) {
          Some(NewSpecifierOrJson::Specifier(new_text)) => {
            replace_specifier(export_all.src, new_text, context)
          }
          Some(NewSpecifierOrJson::Json(_)) => {
            bail_not_supported_json_reexport()?;
          }
          None => {}
        }
      }
      Node::NamedExport(named_export) => {
        if let Some(src) = named_export.src.as_ref() {
          match analyze_specifier(src, context) {
            Some(NewSpecifierOrJson::Specifier(new_text)) => {
              replace_specifier(src, new_text, context)
            }
            Some(NewSpecifierOrJson::Json(_)) => {
              bail_not_supported_json_reexport()?;
            }
            None => {}
          }
        }
      }
      Node::CallExpr(call_expr) => {
        if call_expr.callee.text_fast(context.program) == "import" {
          if let Some(Node::Str(src)) =
            call_expr.args.get(0).map(|a| a.expr.as_node())
          {
            match analyze_specifier(src, context) {
              Some(NewSpecifierOrJson::Specifier(new_text)) => {
                replace_specifier(src, new_text, context)
              }
              Some(NewSpecifierOrJson::Json(json)) => {
                context.text_changes.push(TextChange {
                  span: call_expr.span(),
                  new_text: format!(
                    "Promise.resolve({{ default: JSON.parse(`{}`) }})",
                    json.text.replace("`", "\\`")
                  ),
                });
              }
              None => {}
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

enum NewSpecifierOrJson {
  Specifier(String),
  Json(JsonDetails),
}

struct JsonDetails {
  text: String,
}

fn analyze_specifier(
  str: &Str,
  context: &Context,
) -> Option<NewSpecifierOrJson> {
  let value = str.value().to_string();
  let specifier = context
    .module_graph
    .resolve_dependency(&value, context.specifier)?;
  Some(
    if let Some(bare_specifier) = context.specifier_mappings.get(&specifier) {
      NewSpecifierOrJson::Specifier(bare_specifier.to_string())
    } else {
      let specifier_file_path = context.mappings.get_file_path(&specifier);
      if is_json_file_path(&specifier_file_path) {
        NewSpecifierOrJson::Json(JsonDetails {
          text: context.module_graph.get(&specifier).source().to_string(),
        })
      } else {
        NewSpecifierOrJson::Specifier(get_relative_specifier(
          context.output_file_path,
          specifier_file_path,
        ))
      }
    },
  )
}

fn replace_specifier(str: &Str, new_text: String, context: &mut Context) {
  context.text_changes.push(TextChange {
    span: Span::new(
      str.span().lo + BytePos(1),
      str.span().hi - BytePos(1),
      Default::default(),
    ),
    new_text,
  });
}

fn bail_not_supported_json_reexport() -> Result<()> {
  bail!(concat!(
    "Re-exporting JSON modules has not been implemented. If you need this functionality, please open an issue in dnt's repo.\n\n",
    "As a workaround, consider importing the module with an import declaration then exporting the identifier on a separate statement.",
  ))
}
