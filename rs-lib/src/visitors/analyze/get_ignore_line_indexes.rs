use std::collections::HashSet;

use deno_ast::view::*;

pub fn get_ignore_line_indexes(program: &Program) -> HashSet<usize> {
  let mut result = HashSet::new();
  for comment in program.comment_container().unwrap().all_comments() {
    if comment
      .text
      .trim()
      .to_lowercase()
      .starts_with("deno-shim-ignore")
    {
      if let Some(next_token) = comment.next_token_fast(program) {
        result.insert(next_token.span.lo.start_line_fast(program));
      }
    }
  }
  result
}
