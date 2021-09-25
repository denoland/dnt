// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use deno_ast::ModuleSpecifier;
use deno_graph::create_graph;
use mappings::Mappings;
use text_changes::apply_text_changes;
use visitors::get_deno_global_text_changes;
use visitors::get_module_specifier_text_changes;
use visitors::GetDenoGlobalTextChangesParams;
use visitors::GetModuleSpecifierTextChangesParams;

mod args;
mod loader;
mod mappings;
mod parser;
mod text_changes;
mod utils;
mod visitors;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let args = args::parse_cli_args();
  run(&args).await;
  Ok(())
}

async fn run(args: &args::CliArgs) {
  let mut loader = loader::SourceLoader::new();
  let source_parser = parser::CapturingSourceParser::new();
  let module_graph = create_graph(
    ModuleSpecifier::from_file_path(&args.entry_point).unwrap(),
    &mut loader,
    None,
    None,
    Some(&source_parser),
  )
  .await;

  let local_specifiers = loader.local_specifiers();
  let remote_specifiers = loader.remote_specifiers();

  let mappings =
    Mappings::new(&module_graph, &local_specifiers, &remote_specifiers);

  if local_specifiers.is_empty() {
    panic!("Did not find any local files.");
  }

  // todo: parallelize
  for specifier in local_specifiers
    .into_iter()
    .chain(remote_specifiers.into_iter())
  {
    let parsed_source = source_parser.get_parsed_source(&specifier).unwrap();
    let output_file_path =
      args.out_dir.join(mappings.get_file_path(&specifier));

    let text_changes = parsed_source.with_view(|program| {
      let mut text_changes = get_module_specifier_text_changes(
        &GetModuleSpecifierTextChangesParams {
          specifier: &specifier,
          module_graph: &module_graph,
          mappings: &mappings,
          use_js_extension: args.keep_extensions,
          program: &program,
        },
      );
      text_changes.extend(get_deno_global_text_changes(
        &GetDenoGlobalTextChangesParams {
          program: &program,
          top_level_context: parsed_source.top_level_context(),
        },
      ));
      text_changes
    });

    let result = apply_text_changes(
      parsed_source.source().text().to_string(),
      text_changes,
    );

    std::fs::create_dir_all(output_file_path.parent().unwrap()).unwrap();
    std::fs::write(output_file_path, result).unwrap();
  }
}
