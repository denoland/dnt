// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use deno_ast::swc::ast::Invalid;
use deno_ast::swc::common::DUMMY_SP;
use deno_ast::swc::visit::VisitWith;
use deno_ast::ModuleSpecifier;
use deno_graph::create_graph;
use deno_graph::CapturingSourceParser;
use mappings::Mappings;
use text_changes::apply_text_changes;
use visitors::ModuleSpecifierVisitor;
use visitors::ModuleSpecifierVisitorParams;

mod args;
mod loader;
mod mappings;
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
  let source_parser = CapturingSourceParser::new();
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

  let mappings = Mappings::new(&module_graph, &local_specifiers, &remote_specifiers);

  if local_specifiers.is_empty() {
    panic!("Did not find any local files.");
  }

  for specifier in local_specifiers.into_iter().chain(remote_specifiers.into_iter()) {
    let parsed_source = source_parser.get_parsed_source(&specifier).unwrap();
    let output_file_path = args.out_dir.join(mappings.get_file_path(&specifier));

    let mut module_specifier_visitor =
      ModuleSpecifierVisitor::new(ModuleSpecifierVisitorParams {
        specifier: &specifier,
        module_graph: &module_graph,
        mappings: &mappings,
        use_js_extension: args.keep_extensions,
      });
    parsed_source
      .module()
      .visit_with(&Invalid { span: DUMMY_SP }, &mut module_specifier_visitor);
    let (text_changes, _) = module_specifier_visitor.into_inner();
    let result = apply_text_changes(
      parsed_source.source().text().to_string(),
      text_changes,
    );

    std::fs::create_dir_all(output_file_path.parent().unwrap()).unwrap();
    std::fs::write(output_file_path, result).unwrap();
  }
}
