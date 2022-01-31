// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use once_cell::sync::Lazy;

use crate::graph::ModuleGraph;
use crate::specifiers::Specifiers;
use crate::utils::partition_by_root_specifiers;
use crate::utils::url_to_file_path;

pub struct SyntheticSpecifiers {
  pub polyfills: ModuleSpecifier,
  pub shims: ModuleSpecifier,
}

pub static SYNTHETIC_SPECIFIERS: Lazy<SyntheticSpecifiers> =
  Lazy::new(|| SyntheticSpecifiers {
    polyfills: ModuleSpecifier::parse("dnt://_dnt.polyfills.ts").unwrap(),
    shims: ModuleSpecifier::parse("dnt://_dnt.shims.ts").unwrap(),
  });
pub static SYNTHETIC_TEST_SPECIFIERS: Lazy<SyntheticSpecifiers> =
  Lazy::new(|| SyntheticSpecifiers {
    polyfills: ModuleSpecifier::parse("dnt://_dnt.test_polyfills.ts").unwrap(),
    shims: ModuleSpecifier::parse("dnt://_dnt.test_shims.ts").unwrap(),
  });

pub struct Mappings {
  inner: HashMap<ModuleSpecifier, PathBuf>,
}

impl Mappings {
  pub fn new(
    module_graph: &ModuleGraph,
    specifiers: &Specifiers,
  ) -> Result<Self> {
    let mut mappings = HashMap::new();
    let mut mapped_filepaths_no_ext = HashSet::new();
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
      mappings.insert(
        specifier.clone(),
        get_mapped_file_path(
          relative_file_path.into(),
          &relative_file_path,
          &mut mapped_filepaths_no_ext,
        ),
      );
      if let Some(Component::Normal(first_dir)) =
        relative_file_path.components().next()
      {
        root_local_dirs.insert(PathBuf::from(first_dir));
      }
    }

    let root_remote_specifiers = partition_by_root_specifiers(&specifiers.remote);
    let mut mapped_base_dirs = HashSet::new();
    let deps_path =
      get_unique_path(PathBuf::from("deps"), &mut root_local_dirs);
    for (root, specifiers) in root_remote_specifiers.into_iter() {
      let base_dir = deps_path.join(get_unique_path(
        get_dir_name_for_root(&root),
        &mut mapped_base_dirs,
      ));
      for specifier in specifiers {
        let media_type = module_graph.get(&specifier).media_type;
        let relative = base_dir
          .join(sanitize_filepath(&make_url_relative(&root, &specifier)?));
        mappings.insert(
          specifier,
          get_mapped_file_path(
            media_type,
            &relative,
            &mut mapped_filepaths_no_ext,
          ),
        );
      }
    }

    for (code_specifier, d) in specifiers.types.iter() {
      let to = &d.selected.specifier;
      let file_path = mappings.get(code_specifier).unwrap_or_else(|| {
        panic!(
          "dnt bug - Could not find mapping for types code specifier {}",
          code_specifier
        );
      });
      let new_file_path = file_path.with_extension("d.ts");
      if let Some(past_path) = mappings.insert(to.clone(), new_file_path) {
        panic!(
          "dnt bug - Already had path {} in map when adding declaration file for {}. Adding: {}",
          past_path.display(),
          code_specifier,
          to
        );
      }
    }

    // add the redirects in the graph to the mappings
    for (key, value) in module_graph.redirects() {
      if !mappings.contains_key(key) {
        if let Some(path) = mappings.get(value).map(ToOwned::to_owned) {
          mappings.insert(key.clone(), path);
        } else {
          panic!("dnt bug - Could not find the mapping for {}", value);
        }
      }
    }

    // add the synthetic specifiers even though some of these files won't be created
    fn add_synthetic_specifier(
      mappings: &mut HashMap<ModuleSpecifier, PathBuf>,
      mapped_filepaths_no_ext: &mut HashSet<PathBuf>,
      specifier: &ModuleSpecifier,
    ) {
      debug_assert!(specifier.to_string().starts_with("dnt://"));
      mappings.insert(
        specifier.clone(),
        get_mapped_file_path(
          MediaType::TypeScript,
          &specifier.to_string()["dnt://".len()..],
          mapped_filepaths_no_ext,
        ),
      );
    }

    add_synthetic_specifier(
      &mut mappings,
      &mut mapped_filepaths_no_ext,
      &SYNTHETIC_SPECIFIERS.polyfills,
    );
    add_synthetic_specifier(
      &mut mappings,
      &mut mapped_filepaths_no_ext,
      &SYNTHETIC_TEST_SPECIFIERS.polyfills,
    );
    add_synthetic_specifier(
      &mut mappings,
      &mut mapped_filepaths_no_ext,
      &SYNTHETIC_SPECIFIERS.shims,
    );
    add_synthetic_specifier(
      &mut mappings,
      &mut mapped_filepaths_no_ext,
      &SYNTHETIC_TEST_SPECIFIERS.shims,
    );

    Ok(Mappings { inner: mappings })
  }

  pub fn get_file_path(&self, specifier: &ModuleSpecifier) -> &PathBuf {
    self.inner.get(specifier).unwrap_or_else(|| {
      panic!(
        "dnt bug - Could not find file path for specifier: {}",
        specifier
      )
    })
  }
}

fn get_mapped_file_path(
  media_type: MediaType,
  path: impl AsRef<Path>,
  mapped_filepaths_no_ext: &mut HashSet<PathBuf>,
) -> PathBuf {
  fn without_ext(path: impl AsRef<Path>) -> PathBuf {
    // remove the extension if it's known
    // Ex. url could be `https://deno.land/test/1.2.5`
    // and we don't want to use `1.2`
    let media_type: MediaType = path.as_ref().into();
    if media_type == MediaType::Unknown {
      path.as_ref().into()
    } else {
      path.as_ref().with_extension("")
    }
  }

  let filepath_no_ext =
    get_unique_path(without_ext(path), mapped_filepaths_no_ext);
  let extension = match media_type {
    MediaType::Json => "js",
    _ => &media_type.as_ts_extension()[1..],
  };
  filepath_no_ext.with_extension(
    if let Some(sub_ext) = filepath_no_ext.extension() {
      format!("{}.{}", sub_ext.to_string_lossy(), extension)
    } else {
      extension.to_string()
    },
  )
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
    truncate_str(&result, 30)
      .trim_end_matches('_')
      .trim_end_matches('.')
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
    if is_banned_path_char(c) {
      chars.push('_');
    } else {
      chars.push(c);
    }
  }
  chars.into_iter().collect()
}

fn sanitize_segment(text: &str) -> String {
  let mut chars = Vec::with_capacity(text.len()); // not chars, but good enough
  for c in text.chars() {
    if is_banned_segment_char(c) {
      chars.push('_');
    } else {
      chars.push(c);
    }
  }
  chars.into_iter().collect()
}

fn is_banned_segment_char(c: char) -> bool {
  matches!(c, '/' | '\\') || is_banned_path_char(c)
}

fn is_banned_path_char(c: char) -> bool {
  matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')
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
  use pretty_assertions::assert_eq;

  #[test]
  fn should_get_dir_name_root() {
    run_test("http://deno.land/x/test", "deno.land_x_test");
    run_test("http://localhost", "localhost");
    run_test("http://localhost/test%20test", "localhost_test%20test");
    // will truncate
    run_test(
      "http://localhost/test%20testingtestingtesting",
      "localhost_test%20testingtestin",
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
