// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_graph::Module;
use deno_graph::Resolved;

use crate::declaration_file_resolution::resolve_declaration_file_mappings;
use crate::declaration_file_resolution::DeclarationFileResolution;
use crate::graph::ModuleGraph;
use crate::loader::LoaderSpecifiers;
use crate::MappedSpecifier;

#[derive(Debug)]
pub struct Specifiers {
  pub local: Vec<ModuleSpecifier>,
  pub remote: Vec<ModuleSpecifier>,
  pub types: BTreeMap<ModuleSpecifier, DeclarationFileResolution>,
  pub test_modules: HashSet<ModuleSpecifier>,
  pub main: EnvironmentSpecifiers,
  pub test: EnvironmentSpecifiers,
}

impl Specifiers {
  pub fn has_mapped(&self, specifier: &ModuleSpecifier) -> bool {
    self.main.mapped.contains_key(specifier)
      || self.test.mapped.contains_key(specifier)
  }
}

#[derive(Debug)]
pub struct EnvironmentSpecifiers {
  pub mapped: BTreeMap<ModuleSpecifier, MappedSpecifier>,
}

pub fn get_specifiers(
  entry_points: &[ModuleSpecifier],
  mut specifiers: LoaderSpecifiers,
  module_graph: &ModuleGraph,
  modules: &[&Module],
) -> Result<Specifiers> {
  let mut local_specifiers = Vec::new();
  let mut remote_specifiers = Vec::new();

  let mut modules: BTreeMap<&ModuleSpecifier, &Module> =
    modules.iter().map(|m| (&m.specifier, *m)).collect();

  let mut found_module_specifiers = Vec::new();
  let mut found_mapped_specifiers = BTreeMap::new();

  // search for all the non-test modules
  for entry_point in entry_points.iter() {
    let module = module_graph.get(entry_point);
    let mut pending = vec![&module.specifier];

    while !pending.is_empty() {
      if let Some(module) = pending
        .pop()
        .map(|s| modules.remove(&module_graph.resolve(s)))
        .flatten()
      {
        if let Some(mapped_entry) = specifiers.mapped.remove(&module.specifier)
        {
          found_mapped_specifiers
            .insert(module.specifier.clone(), mapped_entry);
        } else {
          found_module_specifiers.push(module.specifier.clone());

          for dep in module.dependencies.values() {
            if let Some(specifier) = dep.get_code() {
              pending.push(specifier);
            }
            if let Some(specifier) = dep.get_type() {
              pending.push(specifier);
            }
          }
          if let Some((_, Resolved::Ok { specifier, .. })) = &module.maybe_types_dependency
          {
            pending.push(specifier);
          }
        }
      }
    }
  }

  // clear out all the mapped modules
  for specifier in specifiers.mapped.keys() {
    modules.remove(specifier);
  }

  // at this point, the remaining modules are the test modules
  let test_modules = modules;
  let all_modules = test_modules
    .values()
    .copied()
    .chain(found_module_specifiers.iter().map(|s| module_graph.get(s)))
    .collect::<Vec<_>>();

  for module in all_modules.iter() {
    match module.specifier.scheme().to_lowercase().as_str() {
      "file" => local_specifiers.push(module.specifier.clone()),
      "http" | "https" => remote_specifiers.push(module.specifier.clone()),
      _ => {
        anyhow::bail!("Unhandled scheme on url: {}", module.specifier);
      }
    }
  }

  let types = resolve_declaration_file_mappings(module_graph, &all_modules)?;
  let mut declaration_specifiers = HashSet::new();
  for value in types.values() {
    declaration_specifiers.insert(&value.selected.specifier);
    for dep in value.ignored.iter() {
      declaration_specifiers.insert(&dep.specifier);
    }
  }

  ensure_mapped_specifiers_valid(&found_mapped_specifiers, &specifiers.mapped)?;

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
    test_modules: test_modules
      .values()
      .map(|k| k.specifier.clone())
      .collect(),
    main: EnvironmentSpecifiers {
      mapped: found_mapped_specifiers,
    },
    test: EnvironmentSpecifiers {
      mapped: specifiers.mapped,
    },
  })
}

fn ensure_mapped_specifiers_valid(
  mapped_specifiers: &BTreeMap<ModuleSpecifier, MappedSpecifier>,
  test_mapped_specifiers: &BTreeMap<ModuleSpecifier, MappedSpecifier>,
) -> Result<()> {
  let mut specifier_for_name: HashMap<
    String,
    (ModuleSpecifier, MappedSpecifier),
  > = HashMap::new();
  for (from_specifier, mapped_specifier) in mapped_specifiers
    .iter()
    .chain(test_mapped_specifiers.iter())
  {
    if let Some(specifier) = specifier_for_name.get(&mapped_specifier.name) {
      if specifier.1.version != mapped_specifier.version {
        anyhow::bail!("Specifier {} with version {} did not match specifier {} with version {}.",
          specifier.0,
          specifier.1.version.as_deref().unwrap_or("<unknown>"),
          from_specifier,
          mapped_specifier.version.as_deref().unwrap_or("<unknown>"),
        );
      }
    } else {
      specifier_for_name.insert(
        mapped_specifier.name.to_string(),
        (from_specifier.clone(), mapped_specifier.clone()),
      );
    }
  }

  Ok(())
}
