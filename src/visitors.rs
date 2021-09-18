use deno_ast::swc::ast::*;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::Visit;
use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;

use crate::text_changes::TextChange;

pub struct ModuleSpecifierVisitor {
  use_js_extension: bool,
  text_changes: Vec<TextChange>,
}

impl ModuleSpecifierVisitor {
  pub fn new(use_js_extension: bool) -> Self {
    Self {
      use_js_extension,
      text_changes: Vec::new(),
    }
  }

  pub fn take_text_changes(self) -> Vec<TextChange> {
    self.text_changes
  }

  pub fn visit_module_specifier(&mut self, str: &Str) {
    let new_text = match self.use_js_extension {
      true => replace_extension_js(&str.value.to_string()),
      false => strip_extension(&str.value.to_string()),
    };
    self.text_changes.push(TextChange {
      span: Span::new(str.span.lo + BytePos(1), str.span.hi - BytePos(1), Default::default()),
      new_text,
    });

    // todo: actualy implement this stuff. This is extreme laziness :P
    fn replace_extension_js(specifier: &str) -> String {
      specifier.replace(".ts", ".js")
        .replace(".tsx", ".js")
        .replace(".jsx", ".js")
    }

    fn strip_extension(specifier: &str) -> String {
      specifier.replace(".ts", "")
        .replace(".js", "")
        .replace(".tsx", "")
        .replace(".jsx", "")
    }
  }
}

impl Visit for ModuleSpecifierVisitor {
  fn visit_module_item(
    &mut self,
    module_item: &ModuleItem,
    _: &dyn Node
  ) {
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
          },
          _ => {},
        };
      }
      _ => {},
    }
  }
}
