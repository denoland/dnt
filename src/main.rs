// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;

use deno_ast::ModuleSpecifier;
use deno_graph::create_graph;
use futures::executor::block_on;
use transpile::ModuleTarget;

mod args;
mod loader;
mod parser;
mod transpile;
mod transforms;

// Todos
// 1. Support Deno.json to get compiler options.
// 2. Support bundling to single file leaving dependencies unbundled.
// 3. Handle mapping from remote specifiers to bare specifiers and transforming them in the file.
// 4. Handle dynamic imports (at least ones that are statically analyzable and maybe warn on others)

fn main() {
  let args = args::parse_cli_args();
  let future = run_graph(&args);
  block_on(future);
}

async fn run_graph(args: &args::CliArgs) {
  let mut loader = loader::SourceLoader::new();
  let source_parser = parser::SourceParser::new();
  let graph = create_graph(
    ModuleSpecifier::from_file_path(&args.entry_point).unwrap(),
    &mut loader,
    None,
    None,
    Some(&source_parser),
  )
  .await;

  for remote_specifier in loader.remote_specifiers() {
    // todo: construct the mappings from the remote specifiers to bare specifiers here
  }

  let local_specifiers = loader.local_specifiers();
  if local_specifiers.is_empty() {
    panic!("Did not find any local files.");
  }

  // identify the base directory
  let base_dir = get_base_dir(&local_specifiers);

  std::fs::create_dir_all(&args.out_dir).unwrap();

  for local_specifier in local_specifiers.iter() {
    let parsed_source = source_parser.get_parsed_source(local_specifier).unwrap();
    let file_path = ModuleSpecifier::parse(parsed_source.specifier()).unwrap().to_file_path().unwrap();
    let relative_file_path = file_path.strip_prefix(&base_dir).unwrap();

    let mut output_file_path = args.out_dir.join(relative_file_path);
    output_file_path.set_extension(if args.cjs { "js" } else { "mjs" });

    let result = transpile::transpile(&parsed_source, &source_parser, &transpile::EmitOptions {
      module_target: match args.cjs {
        true => ModuleTarget::CommonJs,
        false => ModuleTarget::Esm,
      },
      ..Default::default()
    }).unwrap();

    std::fs::create_dir_all(output_file_path.parent().unwrap()).unwrap();
    std::fs::write(output_file_path, result.0).unwrap();
  }
}

fn get_base_dir(specifiers: &[ModuleSpecifier]) -> PathBuf {
  // todo: should maybe error on windows when the files
  // span different drives...
  let mut base_dir = specifiers[0].to_file_path().unwrap().to_path_buf().parent().unwrap().to_path_buf();
  for specifier in specifiers {
    let file_path = specifier.to_file_path().unwrap();
    let parent_dir = file_path.parent().unwrap();
    if base_dir.starts_with(parent_dir) {
      base_dir = parent_dir.to_path_buf();
    }
  }
  base_dir
}
