// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use d2n::transform;
use d2n::TransformOptions;
use deno_ast::ModuleSpecifier;

mod args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = args::parse_cli_args();
  let output_files = transform(TransformOptions {
    entry_point: ModuleSpecifier::from_file_path(args.entry_point).unwrap(),
    keep_extensions: args.keep_extensions,
    loader: None,
  })
  .await?;

  for output_file in output_files {
    let output_file_path = args.out_dir.join(output_file.file_path);
    std::fs::create_dir_all(output_file_path.parent().unwrap()).unwrap();
    std::fs::write(output_file_path, output_file.file_text).unwrap();
  }

  Ok(())
}
