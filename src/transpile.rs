// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::rc::Rc;

use deno_ast::ModuleSpecifier;
use deno_ast::swc::ast::Program;
use deno_ast::swc::codegen::text_writer::JsWriter;
use deno_ast::swc::codegen::Node;
use deno_ast::swc::common::chain;
use deno_ast::swc::common::FileName;
use deno_ast::swc::common::SourceMap;
use deno_ast::swc::transforms::fixer;
use deno_ast::swc::transforms::helpers;
use deno_ast::swc::transforms::hygiene;
use deno_ast::swc::transforms::pass::Optional;
use deno_ast::swc::transforms::proposals;
use deno_ast::swc::transforms::react;
use deno_ast::swc::transforms::typescript;
use deno_ast::swc::visit::FoldWith;
use deno_ast::ParsedSource;
use swc_ecmascript::transforms::compat;

use crate::parser::SourceParser;
use crate::transforms::ModuleSpecifierFolder;

#[derive(Debug, Clone, Copy)]
pub enum ModuleTarget {
  CommonJs,
  Esm,
}

// todo: would be good to push this down to deno_ast for re-use and
// have the ability to inject transforms into the output

#[derive(Debug, Clone)]
pub enum ImportsNotUsedAsValues {
  Remove,
  Preserve,
  Error,
}

/// Options which can be adjusted when transpiling a module.
#[derive(Debug, Clone)]
pub struct EmitOptions {
  /// Specifies the module target.
  pub module_target: ModuleTarget,
  /// When emitting a legacy decorator, also emit experimental decorator meta
  /// data.  Defaults to `false`.
  pub emit_metadata: bool,
  /// What to do with import statements that only import types i.e. whether to
  /// remove them (`Remove`), keep them as side-effect imports (`Preserve`)
  /// or error (`Error`). Defaults to `Remove`.
  pub imports_not_used_as_values: ImportsNotUsedAsValues,
  /// Should the source map be inlined in the emitted code file, or provided
  /// as a separate file.  Defaults to `true`.
  pub inline_source_map: bool,
  /// Should the sources be inlined in the source map.  Defaults to `true`.
  pub inline_sources: bool,
  // Should a corresponding .map file be created for the output. This should be
  // false if inline_source_map is true. Defaults to `false`.
  pub source_map: bool,
  /// When transforming JSX, what value should be used for the JSX factory.
  /// Defaults to `React.createElement`.
  pub jsx_factory: String,
  /// When transforming JSX, what value should be used for the JSX fragment
  /// factory.  Defaults to `React.Fragment`.
  pub jsx_fragment_factory: String,
  /// Should JSX be transformed or preserved.  Defaults to `true`.
  pub transform_jsx: bool,
}


impl Default for EmitOptions {
  fn default() -> Self {
    EmitOptions {
      module_target: ModuleTarget::Esm,
      emit_metadata: false,
      imports_not_used_as_values: ImportsNotUsedAsValues::Remove,
      inline_source_map: true,
      inline_sources: true,
      source_map: false,
      jsx_factory: "React.createElement".into(),
      jsx_fragment_factory: "React.Fragment".into(),
      transform_jsx: true,
    }
  }
}

/// Transform a TypeScript file into a JavaScript file, based on the supplied
/// options.
///
/// The result is a tuple of the code and optional source map as strings.
pub fn transpile(
  parsed_source: &ParsedSource,
  parser: &SourceParser,
  options: &EmitOptions,
) -> Result<(String, Option<String>), anyhow::Error> {
  let program: Program = (*parsed_source.program()).clone();
  let source_map = Rc::new(SourceMap::default());
  let specifier = ModuleSpecifier::parse(parsed_source.specifier()).unwrap();
  let file_name = FileName::Url(specifier);
  source_map
    .new_source_file(file_name, parsed_source.source().text().to_string());
  let comments = parsed_source.comments().as_single_threaded(); // needs to be mutable

  let jsx_pass = react::react(
    source_map.clone(),
    Some(&comments),
    react::Options {
      pragma: options.jsx_factory.clone(),
      pragma_frag: options.jsx_fragment_factory.clone(),
      // this will use `Object.assign()` instead of the `_extends` helper
      // when spreading props.
      use_builtins: true,
      ..Default::default()
    },
  );
  let mut passes = chain!(
    ModuleSpecifierFolder {
      module_target: options.module_target,
    },
    Optional::new(jsx_pass, options.transform_jsx),
    proposals::decorators::decorators(proposals::decorators::Config {
      legacy: true,
      emit_metadata: options.emit_metadata
    }),
    helpers::inject_helpers(),
    typescript::strip::strip_with_config(strip_config_from_emit_options(
      options
    )),
    fixer(Some(&comments)),
    hygiene(),
    compat::es2021::es2021(),
    compat::es2020::es2020(),
    compat::es2018::es2018(),
    compat::es2017::es2017(),
    compat::es2016::es2016(),
    Optional::new(deno_ast::swc::transforms::modules::common_js(parser.top_level_mark, swc_ecmascript::transforms::modules::common_js::Config {
      strict: true,
      strict_mode: true,
      ..Default::default()
    }, None), matches!(options.module_target, ModuleTarget::CommonJs)),
  );

  let program = deno_ast::swc::common::GLOBALS.set(&parser.globals, || {
    helpers::HELPERS.set(&helpers::Helpers::new(false), || {
      program.fold_with(&mut passes)
    })
  });

  let mut src_map_buf = vec![];
  let mut buf = vec![];
  {
    let writer = Box::new(JsWriter::new(
      source_map.clone(),
      "\n",
      &mut buf,
      Some(&mut src_map_buf),
    ));
    let config = deno_ast::swc::codegen::Config { minify: false };
    let mut emitter = deno_ast::swc::codegen::Emitter {
      cfg: config,
      comments: Some(&comments),
      cm: source_map.clone(),
      wr: writer,
    };
    program.emit_with(&mut emitter)?;
  }
  let mut src = String::from_utf8(buf)?;
  let mut map: Option<String> = None;
  {
    let mut buf = Vec::new();
    source_map
      .build_source_map_from(&mut src_map_buf, None)
      .to_writer(&mut buf)?;

    if options.inline_source_map {
      src.push_str("//# sourceMappingURL=data:application/json;base64,");
      let encoded_map = base64::encode(buf);
      src.push_str(&encoded_map);
    } else {
      map = Some(String::from_utf8(buf)?);
    }
  }
  Ok((src, map))
}

fn strip_config_from_emit_options(
  options: &EmitOptions,
) -> typescript::strip::Config {
  typescript::strip::Config {
    import_not_used_as_values: match options.imports_not_used_as_values {
      ImportsNotUsedAsValues::Remove => {
        typescript::strip::ImportsNotUsedAsValues::Remove
      }
      ImportsNotUsedAsValues::Preserve => {
        typescript::strip::ImportsNotUsedAsValues::Preserve
      }
      // `Error` only affects the type-checking stage. Fall back to `Remove` here.
      ImportsNotUsedAsValues::Error => {
        typescript::strip::ImportsNotUsedAsValues::Remove
      }
    },
    use_define_for_class_fields: true,
    // TODO(bartlomieju): this could be changed to `false` to provide `export {}`
    // in Typescript files without manual changes
    no_empty_export: true,
  }
}
