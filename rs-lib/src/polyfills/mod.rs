// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

#[derive(PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
pub enum Polyfill {
  ObjectHasOwn,
  ErrorCause,
}

pub fn build_polyfill_file(polyfills: &HashSet<Polyfill>) -> Option<String> {
  if polyfills.is_empty() {
    return None;
  }

  let mut file_text = String::new();
  let mut polyfills = polyfills.iter().collect::<Vec<_>>();
  polyfills.sort();

  for polyfill in polyfills {
    match polyfill {
      Polyfill::ObjectHasOwn => {
        file_text.push_str(include_str!("./scripts/object-has-own.ts"));
      }
      Polyfill::ErrorCause => {
        file_text.push_str(include_str!("./scripts/error-cause.ts"));
      }
    }
  }

  Some(file_text)
}
