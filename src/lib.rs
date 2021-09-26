// Copyright 2021 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_graph::create_graph;
use mappings::Mappings;
use text_changes::apply_text_changes;
use visitors::get_deno_global_text_changes;
use visitors::get_module_specifier_text_changes;
use visitors::GetDenoGlobalTextChangesParams;
use visitors::GetModuleSpecifierTextChangesParams;

use loader::DefaultLoader;
pub use loader::Loader;

mod loader;
mod mappings;
mod parser;
mod text_changes;
mod utils;
mod visitors;

pub struct OutputFile {
  pub file_path: PathBuf,
  pub file_text: String,
}

pub struct TransformOptions {
  pub entry_point: PathBuf,
  pub keep_extensions: bool,
  pub loader: Option<Box<dyn Loader>>,
}

pub async fn transform(options: TransformOptions) -> Result<Vec<OutputFile>> {
  let mut loader = loader::SourceLoader::new(
    options
      .loader
      .unwrap_or_else(|| Box::new(DefaultLoader::new())),
  );
  let source_parser = parser::CapturingSourceParser::new();
  let module_graph = create_graph(
    ModuleSpecifier::from_file_path(&options.entry_point).unwrap(),
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
  let mut result = Vec::new();
  for specifier in local_specifiers
    .into_iter()
    .chain(remote_specifiers.into_iter())
  {
    let parsed_source = source_parser.get_parsed_source(&specifier).unwrap();

    let keep_extensions = options.keep_extensions;
    let text_changes = parsed_source.with_view(|program| {
      let mut text_changes = get_module_specifier_text_changes(
        &GetModuleSpecifierTextChangesParams {
          specifier: &specifier,
          module_graph: &module_graph,
          mappings: &mappings,
          use_js_extension: keep_extensions,
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

    let final_file_text = apply_text_changes(
      parsed_source.source().text().to_string(),
      text_changes,
    );

    result.push(OutputFile {
      file_path: mappings.get_file_path(&specifier).to_owned(),
      file_text: final_file_text,
    });
  }

  Ok(result)
}
