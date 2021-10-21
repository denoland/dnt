// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Component;
use std::path::PathBuf;

use anyhow::Result;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use regex::Regex;

use crate::graph::ModuleGraph;
use crate::specifiers::Specifiers;
use crate::utils::url_to_file_path;

lazy_static! {
  static ref HAS_EXTENSION_RE: Regex = Regex::new(r"\.[A-Za-z0-9]*$").unwrap();
}

pub struct Mappings {
  inner: HashMap<ModuleSpecifier, PathBuf>,
}

impl Mappings {
  pub fn new(
    module_graph: &ModuleGraph,
    specifiers: &Specifiers,
  ) -> Result<Self> {
    let mut mappings = HashMap::new();
    let base_dir = get_base_dir(&specifiers.local)?;
    let mut root_local_dirs = HashSet::new();
    for specifier in specifiers.local.iter() {
      let file_path = url_to_file_path(specifier)?;
      let relative_file_path =
        file_path.strip_prefix(&base_dir).map_err(|_| {
          anyhow::anyhow!(
            "Error stripping prefix of {} with base {}",
            file_path.display(),
            base_dir.display()
          )
        })?;
      mappings.insert(specifier.clone(), relative_file_path.to_path_buf());
      if let Some(Component::Normal(first_dir)) =
        relative_file_path.components().next()
      {
        root_local_dirs.insert(PathBuf::from(first_dir));
      }
    }

    let mut root_remote_specifiers: Vec<(
      ModuleSpecifier,
      Vec<(ModuleSpecifier, MediaType)>,
    )> = Vec::new();
    for remote_specifier in specifiers.remote.iter() {
      let media_type = module_graph.get(remote_specifier).media_type;
      let mut found = false;
      for (root_specifier, specifiers) in root_remote_specifiers.iter_mut() {
        if let Some(relative_url) =
          root_specifier.make_relative(remote_specifier)
        {
          // found a new root
          if relative_url.starts_with("../") {
            // todo(dsherret): improve, this was just laziness
            let mut new_root_specifier = root_specifier.clone();
            let mut relative_url = relative_url.as_str();
            while relative_url.starts_with("../") {
              relative_url = &relative_url[3..];
              new_root_specifier = new_root_specifier.join("../").unwrap();
            }
            *root_specifier = new_root_specifier;
          }

          specifiers.push((remote_specifier.clone(), media_type));
          found = true;
          break;
        }
      }
      if !found {
        let root_specifier = remote_specifier
          .join("../")
          .unwrap_or_else(|_| remote_specifier.clone());
        root_remote_specifiers
          .push((root_specifier, vec![(remote_specifier.clone(), media_type)]));
      }
    }

    let mut mapped_base_dirs = HashSet::new();
    let mut mapped_filepaths_no_ext = HashSet::new();
    let deps_path =
      get_unique_path(PathBuf::from("deps"), &mut root_local_dirs);
    for (root, specifiers) in root_remote_specifiers.into_iter() {
      let base_dir = deps_path.join(get_unique_path(
        get_dir_name_for_root(&root),
        &mut mapped_base_dirs,
      ));
      for (specifier, media_type) in specifiers {
        let relative =
          sanitize_filepath(&make_url_relative(&root, &specifier)?);
        let filepath_no_ext = get_unique_path(
          base_dir.join(relative).with_extension(""),
          &mut mapped_filepaths_no_ext,
        );
        let file_path =
          filepath_no_ext.with_extension(&media_type.as_ts_extension()[1..]);
        mappings.insert(specifier, file_path);
      }
    }

    for (code_specifier, d) in specifiers.types.iter() {
      let to = &d.selected.specifier;
      let file_path = mappings.get(code_specifier).unwrap();
      let new_file_path = file_path.with_extension("d.ts");
      if let Some(past_path) = mappings.insert(to.clone(), new_file_path) {
        // this would indicate a programming error
        panic!(
          "Already had path {} in map when adding declaration file for {}. Adding: {}",
          past_path.display(),
          code_specifier,
          to
        );
      }
    }

    Ok(Mappings { inner: mappings })
  }

  pub fn get_file_path(&self, specifier: &ModuleSpecifier) -> &PathBuf {
    self.inner.get(specifier).unwrap_or_else(|| {
      panic!(
        "Programming error. Could not find file path for specifier: {}",
        specifier.to_string()
      )
    })
  }
}

fn get_unique_path(
  mut path: PathBuf,
  unique_set: &mut HashSet<PathBuf>,
) -> PathBuf {
  let original_path = path.to_string_lossy().to_string();
  let mut count = 2;
  while !unique_set.insert(path.clone()) {
    path = PathBuf::from(format!("{}_{}", original_path, count));
    count += 1;
  }
  path
}

fn make_url_relative(
  root: &ModuleSpecifier,
  url: &ModuleSpecifier,
) -> Result<String> {
  let mut url = url.clone();
  url.set_query(None);
  root.make_relative(&url).ok_or_else(|| {
    anyhow::anyhow!(
      "Error making url ({}) relative to root: {}",
      url.to_string(),
      root.to_string()
    )
  })
}

fn get_dir_name_for_root(root: &ModuleSpecifier) -> PathBuf {
  let mut result = String::new();
  if let Some(domain) = root.domain() {
    result.push_str(&sanitize_filepath(domain));
  }
  if let Some(port) = root.port() {
    if !result.is_empty() {
      result.push('_');
    }
    result.push_str(&port.to_string());
  }
  if let Some(segments) = root.path_segments() {
    for segment in segments.filter(|s| !s.is_empty()) {
      if !result.is_empty() {
        result.push('_');
      }
      result.push_str(&sanitize_filepath(segment));
    }
  }

  PathBuf::from(if result.is_empty() {
    "unknown".to_string()
  } else {
    // limit the size of the directory to reduce the chance of max path errors on Windows
    truncate_str(&result.replace(".", "_"), 30)
      .trim_end_matches('_')
      .to_string()
  })
}

fn truncate_str(text: &str, max: usize) -> &str {
  match text.char_indices().nth(max) {
    Some((i, _)) => &text[..i],
    None => text,
  }
}

fn sanitize_filepath(text: &str) -> String {
  let mut chars = Vec::with_capacity(text.len()); // not chars, but good enough
  for c in text.chars() {
    // use an allow list of characters that won't have any issues
    if c.is_alphabetic()
      || c.is_numeric()
      || c.is_whitespace()
      || matches!(c, '_' | '-' | '.' | '/' | '\\')
    {
      chars.push(c);
    } else {
      chars.push('_');
    }
  }
  chars.into_iter().collect()
}

fn get_base_dir(specifiers: &[ModuleSpecifier]) -> Result<PathBuf> {
  // todo(dsherret): should maybe error on windows when the files
  // span different drives...
  let mut base_dir = url_to_file_path(&specifiers[0])?
    .parent()
    .unwrap()
    .to_path_buf();
  for specifier in specifiers {
    let file_path = url_to_file_path(specifier)?;
    let parent_dir = file_path.parent().unwrap();
    if base_dir != parent_dir {
      if base_dir.starts_with(parent_dir) {
        base_dir = parent_dir.to_path_buf();
      } else if base_dir.components().count() == parent_dir.components().count()
      {
        let mut final_path = PathBuf::new();
        for (a, b) in base_dir.components().zip(parent_dir.components()) {
          if a == b {
            final_path.push(a);
          } else {
            break;
          }
        }
        base_dir = final_path;
      }
    }
  }
  Ok(base_dir)
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn should_get_dir_name_root() {
    run_test("http://deno.land/x/test", "deno_land_x_test");
    run_test("http://localhost", "localhost");
    run_test("http://localhost/test%20test", "localhost_test_20test");
    // will truncate
    run_test(
      "http://localhost/test%20testingtestingtesting",
      "localhost_test_20testingtestin",
    );

    fn run_test(specifier: &str, expected: &str) {
      assert_eq!(
        get_dir_name_for_root(&ModuleSpecifier::parse(specifier).unwrap()),
        PathBuf::from(expected)
      );
    }
  }

  #[test]
  fn should_get_base_dir() {
    run_test(
      vec!["file:///project/b/other.ts", "file:///project/a/other.ts"],
      "/project",
    );

    fn run_test(urls: Vec<&str>, expected: &str) {
      let result = get_base_dir(
        &urls
          .into_iter()
          .map(|u| ModuleSpecifier::parse(u).unwrap())
          .collect::<Vec<_>>(),
      )
      .unwrap();
      assert_eq!(result, PathBuf::from(expected));
    }
  }
}
