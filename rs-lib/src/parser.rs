// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::sync::Arc;

use anyhow::Result;
use deno_ast::parse_module;
use deno_ast::Diagnostic;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_ast::ParseParams;
use deno_ast::ParsedSource;
use deno_ast::SourceTextInfo;
use deno_graph::SourceParser;

#[derive(Default)]
pub struct ScopeAnalysisParser {
}

impl ScopeAnalysisParser {
  pub fn new() -> Self {
    ScopeAnalysisParser {}
  }
}

impl SourceParser for ScopeAnalysisParser {
  fn parse_module(
    &self,
    specifier: &ModuleSpecifier,
    source: Arc<String>,
    media_type: MediaType,
  ) -> Result<ParsedSource, Diagnostic> {
    parse_module(ParseParams {
      specifier: specifier.to_string(),
      source: SourceTextInfo::new(source),
      media_type,
      capture_tokens: true, // todo: disable
      scope_analysis: true,
      maybe_syntax: None,
    })
  }
}
