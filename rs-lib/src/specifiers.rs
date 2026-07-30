// Copyright 2018-2024 the Deno authors. MIT license.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::Result;
use deno_ast::ModuleSpecifier;
use deno_graph::Module;
use deno_graph::Resolution;
use deno_semver::npm::NpmPackageReqReference;

use crate::declaration_file_resolution::resolve_declaration_file_mappings;
use crate::declaration_file_resolution::DeclarationFileResolution;
use crate::graph::display_specifier;
use crate::graph::JsrSpecifierMappings;
use crate::graph::ModuleGraph;
use crate::loader::LoaderSpecifiers;
use crate::PackageMappedSpecifier;

#[derive(Debug)]
pub struct Specifiers {
  pub local: Vec<ModuleSpecifier>,
  pub remote: Vec<ModuleSpecifier>,
  pub types: BTreeMap<ModuleSpecifier, DeclarationFileResolution>,
  /// Packages that provide the declaration files of a mapped package,
  /// keyed by package name (ex. an `@types/` package served by a cdn).
  pub types_packages: BTreeMap<String, PackageMappedSpecifier>,
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
  pub mapped: BTreeMap<ModuleSpecifier, PackageMappedSpecifier>,
}

pub fn get_specifiers<'a>(
  entry_points: &[ModuleSpecifier],
  mut specifiers: LoaderSpecifiers,
  jsr_specifier_mappings: &JsrSpecifierMappings,
  module_graph: &ModuleGraph,
  modules: impl Iterator<Item = &'a Module>,
) -> Result<Specifiers> {
  let mut local_specifiers = Vec::new();
  let mut remote_specifiers = Vec::new();

  let mut modules: BTreeMap<&ModuleSpecifier, &Module> =
    modules.map(|m| (m.specifier(), m)).collect();

  let mut found_module_specifiers = Vec::new();
  let mut found_mapped_specifiers = BTreeMap::new();

  // search for all the non-test modules
  for entry_point in entry_points.iter() {
    let module = module_graph.get(entry_point);
    let mut pending = vec![module.specifier()];

    while !pending.is_empty() {
      if let Some(module) = pending
        .pop()
        .and_then(|s| modules.remove(&module_graph.resolve(s)))
      {
        if let Some(mapped_entry) =
          specifiers.mapped_packages.remove(module.specifier())
        {
          found_mapped_specifiers
            .insert(module.specifier().clone(), mapped_entry);
        } else if let Ok(npm_specifier) =
          deno_semver::npm::NpmPackageReqReference::from_specifier(
            module.specifier(),
          )
        {
          found_mapped_specifiers.insert(
            module.specifier().clone(),
            PackageMappedSpecifier::from_npm_specifier(&npm_specifier),
          );
        } else {
          found_module_specifiers.push(module.specifier().clone());

          if let Some(module) = module.js() {
            for dep in module.dependencies.values() {
              if let Some(specifier) = dep.get_code() {
                pending.push(specifier);
              }
              if let Some(specifier) = dep.get_type() {
                pending.push(specifier);
              }
            }
            if let Some(deno_graph::TypesDependency {
              dependency: Resolution::Ok(resolved),
              ..
            }) = &module.maybe_types_dependency
            {
              pending.push(&resolved.specifier);
            }
          }
        }
      }
    }
  }

  // clear out all the mapped modules
  for specifier in specifiers.mapped_packages.keys() {
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
    match module {
      Module::Js(_) | Module::Json(_) => {
        match module.specifier().scheme().to_lowercase().as_str() {
          "file" => local_specifiers.push(module.specifier().clone()),
          "http" | "https" => {
            remote_specifiers.push(module.specifier().clone())
          }
          _ => {
            anyhow::bail!("Unhandled scheme on url: {}", module.specifier());
          }
        }
      }
      Module::Npm(_) | Module::Node(_) => {
        // ignore
      }
      Module::Wasm(_) => {
        anyhow::bail!(
          "Not implemented support for Wasm modules: {}",
          module.specifier()
        );
      }
      Module::External(module) => {
        let specifier = &module.specifier;
        if let Ok(npm_specifier) =
          NpmPackageReqReference::from_specifier(specifier)
        {
          if !found_mapped_specifiers.contains_key(specifier) {
            specifiers.mapped_packages.insert(
              specifier.clone(),
              PackageMappedSpecifier::from_npm_specifier(&npm_specifier),
            );
          }
        }
      }
    }
  }

  let declaration_files = resolve_declaration_file_mappings(
    module_graph,
    &all_modules,
    &found_mapped_specifiers,
  )?;
  let types = declaration_files.mappings;
  let mut declaration_specifiers = declaration_files.types_package_files;
  // a declaration file that a module imports directly still needs to be
  // in the output even when a types package provides it
  for specifier in get_imported_specifiers(module_graph, &all_modules) {
    declaration_specifiers.remove(&specifier);
  }
  for value in types.values() {
    declaration_specifiers.insert(value.selected.specifier.clone());
    for dep in value.ignored.iter() {
      declaration_specifiers.insert(dep.specifier.clone());
    }
  }

  jsr_specifier_mappings.unify_versions(
    found_mapped_specifiers
      .iter_mut()
      .chain(specifiers.mapped_packages.iter_mut()),
  );
  ensure_package_mapped_specifiers_valid(
    &found_mapped_specifiers,
    &specifiers.mapped_packages,
  )?;

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
    types_packages: declaration_files.types_packages,
    test_modules: test_modules
      .values()
      .map(|k| k.specifier().clone())
      .collect(),
    main: EnvironmentSpecifiers {
      mapped: found_mapped_specifiers,
    },
    test: EnvironmentSpecifiers {
      mapped: specifiers.mapped_packages,
    },
  })
}

/// Gets the specifiers that the modules import in their code, which
/// excludes the declaration files they only specify types with.
fn get_imported_specifiers(
  module_graph: &ModuleGraph,
  modules: &[&Module],
) -> HashSet<ModuleSpecifier> {
  let mut specifiers = HashSet::new();
  for module in modules.iter().filter_map(|m| m.js()) {
    for dep in module.dependencies.values() {
      if let Some(specifier) = dep.get_code() {
        specifiers.insert(module_graph.resolve(specifier).clone());
      }
    }
  }
  specifiers
}

/// Gets the local and remote modules that are only reachable from the
/// provided roots, which is used to keep a binary entrypoint's modules out
/// of the script output.
pub fn get_exclusively_reachable(
  module_graph: &ModuleGraph,
  roots: &[ModuleSpecifier],
  other_roots: &[ModuleSpecifier],
) -> HashSet<ModuleSpecifier> {
  let mut reachable = get_reachable(module_graph, roots);
  for specifier in get_reachable(module_graph, other_roots) {
    reachable.remove(&specifier);
  }
  reachable
}

fn get_reachable(
  module_graph: &ModuleGraph,
  roots: &[ModuleSpecifier],
) -> HashSet<ModuleSpecifier> {
  let mut found = HashSet::new();
  let mut pending = roots.iter().cloned().collect::<Vec<_>>();
  while let Some(specifier) = pending.pop() {
    let specifier = module_graph.resolve(&specifier).clone();
    if !found.insert(specifier.clone()) {
      continue;
    }
    let Some(module) = module_graph.try_get(&specifier).and_then(|m| m.js())
    else {
      continue;
    };
    for dep in module.dependencies.values() {
      if let Some(specifier) = dep.get_code() {
        pending.push(specifier.clone());
      }
      if let Some(specifier) = dep.get_type() {
        pending.push(specifier.clone());
      }
    }
    if let Some(deno_graph::TypesDependency {
      dependency: Resolution::Ok(resolved),
      ..
    }) = &module.maybe_types_dependency
    {
      pending.push(resolved.specifier.clone());
    }
  }
  found
}

fn ensure_package_mapped_specifiers_valid(
  mapped_specifiers: &BTreeMap<ModuleSpecifier, PackageMappedSpecifier>,
  test_mapped_specifiers: &BTreeMap<ModuleSpecifier, PackageMappedSpecifier>,
) -> Result<()> {
  let mut specifier_for_name: HashMap<
    String,
    (ModuleSpecifier, PackageMappedSpecifier),
  > = HashMap::new();
  for (from_specifier, mapped_specifier) in mapped_specifiers
    .iter()
    .chain(test_mapped_specifiers.iter())
  {
    if let Some(specifier) = specifier_for_name.get(&mapped_specifier.name) {
      if specifier.1.version != mapped_specifier.version {
        anyhow::bail!("Specifier {} with version {} did not match specifier {} with version {}.",
          display_specifier(&specifier.0),
          specifier.1.version.as_deref().unwrap_or("<unknown>"),
          display_specifier(from_specifier),
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
