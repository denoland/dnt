use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_graph::Module;

use crate::declaration_file_resolution::resolve_declaration_file_mappings;
use crate::declaration_file_resolution::DeclarationFileResolution;
use crate::graph::ModuleGraph;
use crate::loader::LoaderSpecifiers;
use crate::loader::MappedSpecifierEntry;

pub struct Specifiers {
  pub local: Vec<ModuleSpecifier>,
  pub remote: Vec<ModuleSpecifier>,
  pub types: BTreeMap<ModuleSpecifier, DeclarationFileResolution>,
  pub found_ignored: HashSet<ModuleSpecifier>,
  pub mapped: BTreeMap<ModuleSpecifier, MappedSpecifierEntry>,
}

pub fn get_specifiers(
  specifiers: LoaderSpecifiers,
  module_graph: &ModuleGraph,
  modules: &[&Module],
) -> Result<Specifiers> {
  let mut local_specifiers = Vec::new();
  let mut remote_specifiers = Vec::new();

  for module in modules.iter() {
    match module.specifier.scheme().to_lowercase().as_str() {
      "file" => local_specifiers.push(module.specifier.clone()),
      "http" | "https" => remote_specifiers.push(module.specifier.clone()),
      _ => {
        anyhow::bail!("Unhandled scheme on url: {}", module.specifier);
      }
    }
  }

  let types = resolve_declaration_file_mappings(module_graph, &modules)?;
  let mut declaration_specifiers = HashSet::new();
  for value in types.values() {
    declaration_specifiers.insert(&value.selected.specifier);
    for dep in value.ignored.iter() {
      declaration_specifiers.insert(&dep.specifier);
    }
  }

  Ok(Specifiers {
    local: local_specifiers
      .into_iter()
      .filter(|l| !declaration_specifiers.contains(&l))
      .collect(),
    remote: remote_specifiers
      .into_iter()
      .filter(|l| !declaration_specifiers.contains(&l))
      .collect(),
    types,
    found_ignored: specifiers.found_ignored,
    mapped: get_mapped(specifiers.mapped)?,
  })
}

fn get_mapped(
  mapped_specifiers: Vec<MappedSpecifierEntry>,
) -> Result<BTreeMap<ModuleSpecifier, MappedSpecifierEntry>> {
  let mut specifier_for_name: HashMap<String, MappedSpecifierEntry> =
    HashMap::new();
  let mut result = BTreeMap::new();
  for mapped_specifier in mapped_specifiers {
    if let Some(specifier) =
      specifier_for_name.get(&mapped_specifier.to_specifier)
    {
      if specifier.version != mapped_specifier.version {
        anyhow::bail!("Specifier {} with version {} did not match specifier {} with version {}.",
          specifier.from_specifier,
          specifier.version.as_ref().map(|v| v.as_str()).unwrap_or("<unknown>"),
          mapped_specifier.from_specifier,
          mapped_specifier.version.as_ref().map(|v| v.as_str()).unwrap_or("<unknown>"),
        );
      }
    } else {
      specifier_for_name.insert(
        mapped_specifier.to_specifier.to_string(),
        mapped_specifier.clone(),
      );
      result.insert(mapped_specifier.from_specifier.clone(), mapped_specifier);
    }
  }

  Ok(result)
}
