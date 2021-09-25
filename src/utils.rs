// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::path::Path;
use std::path::PathBuf;

pub fn get_relative_path(
  from: impl AsRef<Path>,
  to: impl AsRef<Path>,
) -> PathBuf {
  pathdiff::diff_paths(to, from.as_ref().parent().unwrap()).unwrap()
}
