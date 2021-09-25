// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;

use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;
use deno_ast::view::*;
use deno_ast::ModuleSpecifier;
use deno_graph::ModuleGraph;

use crate::mappings::Mappings;
use crate::text_changes::TextChange;
use crate::utils::get_relative_path;

pub struct GetModuleSpecifierTextChangesParams<'a> {
  pub specifier: &'a ModuleSpecifier,
  pub module_graph: &'a ModuleGraph,
  pub use_js_extension: bool,
  pub mappings: &'a Mappings,
  pub program: &'a Program<'a>,
}

struct Context<'a> {
  specifier: &'a ModuleSpecifier,
  module_graph: &'a ModuleGraph,
  use_js_extension: bool,
  mappings: &'a Mappings,
  output_file_path: &'a PathBuf,
  program: &'a Program<'a>,
  text_changes: Vec<TextChange>,
}

pub fn get_module_specifier_text_changes<'a>(
  params: &GetModuleSpecifierTextChangesParams<'a>,
) -> Vec<TextChange> {
  let mut context = Context {
    specifier: params.specifier,
    module_graph: params.module_graph,
    use_js_extension: params.use_js_extension,
    mappings: params.mappings,
    output_file_path: params.mappings.get_file_path(params.specifier),
    program: params.program,
    text_changes: Vec::new(),
  };

  for child in params.program.children() {
    match child {
      Node::ImportDecl(import_decl) => {
        visit_module_specifier(&import_decl.src, &mut context);
      }
      Node::ExportAll(export_all) => {
        visit_module_specifier(&export_all.src, &mut context);
      }
      Node::NamedExport(named_export) => {
        if let Some(src) = named_export.src.as_ref() {
          visit_module_specifier(src, &mut context);
        }
      }
      _ => {}
    }
  }

  context.text_changes
}

fn visit_module_specifier(str: &Str, context: &mut Context) {
  let value = str.value().to_string();
  let specifier = match context
    .module_graph
    .resolve_dependency(&value, &context.specifier)
  {
    Some(specifier) => specifier,
    // todo: error instead of panic
    None => panic!("Could not resolve specifier: {}", value),
  };
  let specifier_file_path = context.mappings.get_file_path(&specifier);
  let relative_path =
    get_relative_path(context.output_file_path, specifier_file_path);
  let relative_path_str = if context.use_js_extension {
    relative_path.with_extension("js")
  } else {
    relative_path.with_extension("")
  }
  .to_string_lossy()
  .to_string()
  .replace("\\", "/");
  let new_text = if relative_path_str.starts_with("../")
    || relative_path_str.starts_with("./")
  {
    relative_path_str
  } else {
    format!("./{}", relative_path_str)
  };

  context.text_changes.push(TextChange {
    span: Span::new(
      str.span().lo + BytePos(1),
      str.span().hi - BytePos(1),
      Default::default(),
    ),
    new_text,
  });
}
