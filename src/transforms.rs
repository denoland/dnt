use deno_ast::swc::ast::*;
use deno_ast::swc::visit::noop_fold_type;
use deno_ast::swc::visit::Fold;

use crate::transpile::ModuleTarget;

pub struct ModuleSpecifierFolder {
  pub module_target: ModuleTarget,
}

impl ModuleSpecifierFolder {
  pub fn handle_str(&self, str: &mut Str) {
    str.value = match self.module_target {
      ModuleTarget::Esm => replace_extension_mjs(&str.value.to_string()).into(),
      ModuleTarget::CommonJs => strip_extension(&str.value.to_string()).into(),
    };
    str.kind = StrKind::Synthesized;

    // todo: actualy implement this stuff. This is extreme laziness :P
    fn replace_extension_mjs(specifier: &str) -> String {
      specifier.replace(".ts", ".mjs")
        .replace(".tsx", ".mjs")
        .replace(".jsx", ".mjs")
    }

    fn strip_extension(specifier: &str) -> String {
      specifier.replace(".ts", "")
        .replace(".js", "")
        .replace(".tsx", "")
        .replace(".jsx", "")
    }
  }
}

impl Fold for ModuleSpecifierFolder {
  noop_fold_type!(); // skip typescript specific nodes

  fn fold_module_item(
    &mut self,
    module_item: ModuleItem,
  ) -> ModuleItem {
    match module_item {
      ModuleItem::ModuleDecl(module_decl) => {
        ModuleItem::ModuleDecl(match module_decl {
          ModuleDecl::Import(mut import_decl) => {
            self.handle_str(&mut import_decl.src);
            ModuleDecl::Import(import_decl)
          }
          ModuleDecl::ExportAll(mut export_all) => {
            self.handle_str(&mut export_all.src);
            ModuleDecl::ExportAll(export_all)
          }
          ModuleDecl::ExportNamed(mut export_named) => {
            if let Some(src) = export_named.src.as_mut() {
              self.handle_str(src);
            }
            ModuleDecl::ExportNamed(export_named)
          }
          _ => module_decl,
        })
      }
      _ => module_item,
    }
  }
}
