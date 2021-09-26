use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use deno_ast::parse_module;
use deno_ast::Diagnostic;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_ast::ParseParams;
use deno_ast::ParsedSource;
use deno_ast::SourceTextInfo;
use deno_graph::SourceParser;

#[derive(Default)]
pub struct CapturingSourceParser {
  modules: RefCell<HashMap<ModuleSpecifier, ParsedSource>>,
}

impl CapturingSourceParser {
  pub fn new() -> Self {
    Self {
      modules: RefCell::new(HashMap::new()),
    }
  }

  pub fn get_parsed_source(
    &self,
    specifier: &ModuleSpecifier,
  ) -> Option<ParsedSource> {
    self.modules.borrow().get(specifier).map(|m| m.to_owned())
  }
}

impl SourceParser for CapturingSourceParser {
  fn parse_module(
    &self,
    specifier: &ModuleSpecifier,
    source: Arc<String>,
    media_type: MediaType,
  ) -> Result<ParsedSource, Diagnostic> {
    let module = parse_module(ParseParams {
      specifier: specifier.to_string(),
      source: SourceTextInfo::new(source),
      media_type,
      capture_tokens: true, // todo: disable
      scope_analysis: true,
      maybe_syntax: None,
    })?;

    self
      .modules
      .borrow_mut()
      .insert(specifier.clone(), module.clone());

    Ok(module)
  }
}
