// Copyright 2018-2024 the Deno authors. MIT license.

use std::collections::BTreeMap;
use std::collections::HashSet;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_graph::JsModule;
use deno_graph::Module;
use deno_graph::Resolution;

use crate::graph::ModuleGraph;
use crate::PackageMappedSpecifier;

#[derive(Debug)]
pub struct DeclarationFileResolution {
  pub selected: TypesDependency,
  /// Specified declaration dependencies that were ignored.
  pub ignored: Vec<TypesDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypesDependency {
  /// The module being specified.
  pub specifier: ModuleSpecifier,
  /// The module that specified the specifier.
  pub referrer: ModuleSpecifier,
}

pub fn resolve_declaration_file_mappings(
  module_graph: &ModuleGraph,
  modules: &[&Module],
  mapped_specifiers: &BTreeMap<ModuleSpecifier, PackageMappedSpecifier>,
) -> Result<BTreeMap<ModuleSpecifier, DeclarationFileResolution>> {
  let mut type_dependencies = BTreeMap::new();

  for module in modules.iter().filter_map(|m| m.js()) {
    fill_types_for_module(module_graph, module, &mut type_dependencies)?;
  }

  // get the resolved type dependencies
  let mut mappings = BTreeMap::new();
  for (code_specifier, deps) in type_dependencies.into_iter() {
    // if this type_dependency is mapped, then pass it.
    if mapped_specifiers.contains_key(&code_specifier) {
      continue;
    }

    let deps = deps.into_iter().collect::<Vec<_>>();
    let selected_dep =
      select_best_types_dep(module_graph, &code_specifier, &deps);

    // get the declaration file specifiers that weren't used
    let mut ignored = deps
      .into_iter()
      .filter(|d| d.specifier != selected_dep.specifier)
      .collect::<Vec<_>>();
    ignored.sort();

    mappings.insert(
      code_specifier,
      DeclarationFileResolution {
        selected: selected_dep,
        ignored,
      },
    );
  }

  Ok(mappings)
}

/// This resolution process works as follows:
///
/// 1. Prefer using a declaration file specified in local code over remote. This allows the user
///    to override what is potentially done remotely and in the worst case provide their own declaration file.
/// 2. Next prefer when the referrer is from the declaration file itself (ex. x-deno-types header)
/// 3. Finally use the declaration file that is the largest.
fn select_best_types_dep(
  module_graph: &ModuleGraph,
  code_specifier: &ModuleSpecifier,
  deps: &[TypesDependency],
) -> TypesDependency {
  assert!(!deps.is_empty());
  let mut selected_dep = &deps[0];
  for dep in &deps[1..] {
    let is_dep_referrer_local = dep.referrer.scheme() == "file";
    let is_dep_referrer_code = &dep.referrer == code_specifier;

    let is_selected_referrer_local = selected_dep.referrer.scheme() == "file";
    let is_selected_referrer_code = &selected_dep.referrer == code_specifier;

    let should_replace = if is_dep_referrer_local && !is_selected_referrer_local
    {
      true
    } else if is_dep_referrer_local == is_selected_referrer_local {
      if is_selected_referrer_code {
        false
      } else if is_dep_referrer_code {
        true
      } else if let Some(dep_source) =
        module_graph.get(&dep.specifier).js().map(|m| &m.source)
      {
        // as a last resort, use the declaration file that's the largest
        if let Some(selected_source) = module_graph
          .get(&selected_dep.specifier)
          .js()
          .map(|m| &m.source)
        {
          dep_source.len() > selected_source.len()
        } else {
          true
        }
      } else {
        false
      }
    } else {
      false
    };
    if should_replace {
      selected_dep = dep;
    }
  }
  selected_dep.clone()
}

fn fill_types_for_module(
  module_graph: &ModuleGraph,
  module: &JsModule,
  type_dependencies: &mut BTreeMap<ModuleSpecifier, HashSet<TypesDependency>>,
) -> Result<()> {
  // check for the module specifying its type dependency
  match &module.maybe_types_dependency {
    Some(deno_graph::TypesDependency {
      specifier: text,
      dependency: Resolution::Err(err),
    }) => anyhow::bail!(
      "Error resolving types for {} with reference {}. {}",
      module.specifier,
      text,
      err.to_string()
    ),
    Some(deno_graph::TypesDependency {
      dependency: Resolution::Ok(resolved),
      ..
    }) => {
      add_type_dependency(
        module,
        &module.specifier,
        &resolved.specifier,
        type_dependencies,
      );
    }
    _ => {}
  }

  // find any @deno-types
  for dep in module.dependencies.values() {
    if let Some(type_dep) = dep.get_type() {
      if let Some(code_dep) = dep.get_code() {
        if module_graph
          .get(type_dep)
          .js()
          .map(|module| is_declaration_file(module.media_type))
          .unwrap_or(false)
        {
          add_type_dependency(module, code_dep, type_dep, type_dependencies);
        }
      }
    }
  }

  return Ok(());

  fn add_type_dependency(
    module: &JsModule,
    code_specifier: &ModuleSpecifier,
    type_specifier: &ModuleSpecifier,
    type_dependencies: &mut BTreeMap<ModuleSpecifier, HashSet<TypesDependency>>,
  ) {
    // if the code specifier is the same as the type specifier, then no
    // mapping is necessary
    if code_specifier == type_specifier {
      return;
    }

    type_dependencies
      .entry(code_specifier.clone())
      .or_default()
      .insert(TypesDependency {
        referrer: module.specifier.clone(),
        specifier: type_specifier.clone(),
      });
  }
}

fn is_declaration_file(media_type: deno_ast::MediaType) -> bool {
  // todo: use media_type.is_declaration() in deno_ast once available
  use deno_ast::MediaType::*;
  match media_type {
    Dts | Dmts | Dcts => true,
    JavaScript | Jsx | Mjs | Cjs | TypeScript | Mts | Cts | Tsx | Json
    | Wasm | TsBuildInfo | SourceMap | Unknown => false,
  }
}
