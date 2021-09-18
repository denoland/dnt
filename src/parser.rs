use std::cell::RefCell;
use std::collections::HashMap;

use deno_ast::ModuleSpecifier;
use deno_ast::ParseParams;
use deno_ast::ParsedSource;
use deno_ast::SourceTextInfo;
use deno_ast::parse_program_with_post_process;
use deno_ast::swc::common::Globals;
use deno_ast::swc::common::Mark;
use deno_ast::swc::transforms::resolver::ts_resolver;
use deno_ast::swc::visit::FoldWith;

pub struct SourceParser {
  pub globals: Globals,
  pub top_level_mark: Mark,
  modules: RefCell<HashMap<ModuleSpecifier, ParsedSource>>,
}

impl SourceParser {
  pub fn new() -> Self {
    let globals = Globals::new();
    let top_level_mark = deno_ast::swc::common::GLOBALS
      .set(&globals, || Mark::fresh(Mark::root()));
    Self {
      globals,
      top_level_mark,
      modules: RefCell::new(HashMap::new()),
    }
  }

  pub fn get_parsed_source(&self, module_specifier: &ModuleSpecifier) -> Option<ParsedSource> {
    self.modules.borrow().get(module_specifier).map(ToOwned::to_owned)
  }
}

impl deno_graph::SourceParser for SourceParser {
    fn parse_module(
    &self,
    specifier: &deno_graph::ModuleSpecifier,
    source: std::sync::Arc<String>,
    media_type: deno_ast::MediaType,
  ) -> anyhow::Result<deno_ast::ParsedSource, deno_ast::Diagnostic> {
    // todo: add parse_module_with_post_process in deno_ast as everything will be modules
    let result = parse_program_with_post_process(ParseParams {
      specifier: specifier.to_string(),
      source: SourceTextInfo::new(source),
      media_type,
      capture_tokens: false,
      maybe_syntax: None,
    }, |program| {
      deno_ast::swc::common::GLOBALS.set(&self.globals, || {
        program.fold_with(&mut ts_resolver(self.top_level_mark))
      })
    })?;
    self.modules.borrow_mut().insert(specifier.clone(), result.clone());
    Ok(result)
  }
}
