// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;

use deno_ast::ModuleSpecifier;
use deno_ast::swc::ast::*;
use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::Visit;
use deno_graph::ModuleGraph;

use crate::mappings::Mappings;
use crate::text_changes::TextChange;
use crate::utils::get_relative_path;

pub struct ModuleSpecifierVisitorParams<'a> {
  pub specifier: &'a ModuleSpecifier,
  pub module_graph: &'a ModuleGraph,
  pub use_js_extension: bool,
  pub mappings: &'a Mappings,
}

pub struct ModuleSpecifierVisitor<'a> {
  specifier: &'a ModuleSpecifier,
  output_file_path: &'a PathBuf,
  module_graph: &'a ModuleGraph,
  use_js_extension: bool,
  mappings: &'a Mappings,
  text_changes: Vec<TextChange>,
  relative_specifiers: Vec<String>,
}

impl<'a> ModuleSpecifierVisitor<'a> {
  pub fn new(params: ModuleSpecifierVisitorParams<'a>) -> Self {
    let output_file_path = params.mappings.get_file_path(params.specifier);
    Self {
      specifier: params.specifier,
      module_graph: params.module_graph,
      use_js_extension: params.use_js_extension,
      mappings: params.mappings,
      text_changes: Vec::new(),
      relative_specifiers: Vec::new(),
      output_file_path,
    }
  }

  pub fn into_inner(self) -> (Vec<TextChange>, Vec<String>) {
    (self.text_changes, self.relative_specifiers)
  }

  fn visit_module_specifier(&mut self, str: &Str) {
    let value = str.value.to_string();
    let specifier = match self.module_graph.resolve_dependency(&value, &self.specifier) {
      Some(specifier) => specifier,
      // todo: error instead of panic
      None => panic!("Could not resolve specifier: {}", value),
    };
    let specifier_file_path = self.mappings.get_file_path(&specifier);
    let relative_path = get_relative_path(self.output_file_path, specifier_file_path);
    let relative_path_str = if self.use_js_extension {
      relative_path.with_extension("js")
    } else {
      relative_path.with_extension("")
    }.to_string_lossy().to_string().replace("\\", "/");
    let new_text = if relative_path_str.starts_with("../") || relative_path_str.starts_with("./") {
      relative_path_str
    } else {
      format!("./{}", relative_path_str)
    };

    self.text_changes.push(TextChange {
      span: Span::new(
        str.span.lo + BytePos(1),
        str.span.hi - BytePos(1),
        Default::default(),
      ),
      new_text,
    });
  }
}

impl<'a> Visit for ModuleSpecifierVisitor<'a> {
  fn visit_module_item(&mut self, module_item: &ModuleItem, _: &dyn Node) {
    match module_item {
      ModuleItem::ModuleDecl(module_decl) => {
        match module_decl {
          ModuleDecl::Import(import_decl) => {
            self.visit_module_specifier(&import_decl.src);
          }
          ModuleDecl::ExportAll(export_all) => {
            self.visit_module_specifier(&export_all.src);
          }
          ModuleDecl::ExportNamed(export_named) => {
            if let Some(src) = export_named.src.as_ref() {
              self.visit_module_specifier(src);
            }
          }
          _ => {}
        };
      }
      _ => {}
    }
  }
}
