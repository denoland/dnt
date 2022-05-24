use std::collections::HashSet;

use deno_ast::RootNode;
use deno_ast::SourceRangedForSpanned;
use deno_ast::view::*;

pub struct IgnoredLineIndexes {
  pub warnings: Vec<String>,
  pub line_indexes: HashSet<usize>,
}

pub fn get_ignore_line_indexes(
  specifier: &str,
  program: &Program,
) -> IgnoredLineIndexes {
  let mut warnings = Vec::new();
  let mut line_indexes = HashSet::new();
  for comment in program.comment_container().all_comments() {
    let lowercase_text = comment.text.trim().to_lowercase();
    let starts_with_deno_shim_ignore =
      lowercase_text.starts_with("deno-shim-ignore");
    if starts_with_deno_shim_ignore
      || lowercase_text.starts_with("dnt-shim-ignore")
    {
      if let Some(next_token) = comment.next_token_fast(program) {
        line_indexes.insert(next_token.span.lo.start_line_fast(program));
      }
    }
    if starts_with_deno_shim_ignore {
      warnings.push(
        format!("deno-shim-ignore has been renamed to dnt-shim-ignore. Please rename it in {}", specifier)
      );
    }
  }
  IgnoredLineIndexes {
    warnings,
    line_indexes,
  }
}
