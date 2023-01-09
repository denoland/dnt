// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Result;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use once_cell::sync::Lazy;

use crate::graph::ModuleGraph;
use crate::specifiers::Specifiers;
use crate::utils::get_unique_path;
use crate::utils::partition_by_root_specifiers;
use crate::utils::url_to_file_path;
use crate::utils::with_extension;

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
          relative_file_path,
          &mut mapped_filepaths_no_ext,
        ),
      );
      if let Some(Component::Normal(first_dir)) =
        relative_file_path.components().next()
      {
        root_local_dirs.insert(first_dir.to_string_lossy().to_lowercase());
      }
    }

    let deps_path =
      get_unique_path(PathBuf::from("deps"), &mut root_local_dirs);
    for (specifier, suggested_path) in
      remote_specifiers_to_paths(specifiers.remote.iter())
    {
      let media_type = module_graph.get(&specifier).media_type;
      mappings.insert(
        specifier,
        get_mapped_file_path(
          media_type,
          &deps_path.join(suggested_path),
          &mut mapped_filepaths_no_ext,
        ),
      );
    }

    for (code_specifier, d) in specifiers.types.iter() {
      let to = &d.selected.specifier;
      let file_path = mappings.get(code_specifier).unwrap_or_else(|| {
        panic!(
          "dnt bug - Could not find mapping for types code specifier {}",
          code_specifier
        );
      });
      let new_file_path = with_extension(file_path, "d.ts");
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
      mapped_filepaths_no_ext: &mut HashSet<String>,
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
      panic!("Could not find file path for specifier: {}", specifier)
    })
  }
}

/// Takes a group of remote specifiers for the provided base directory
/// and gets their output paths.
fn remote_specifiers_to_paths<'a>(
  specifiers: impl Iterator<Item = &'a ModuleSpecifier>,
) -> Vec<(ModuleSpecifier, PathBuf)> {
  // Use a constant value, because we want the code to be portable
  // when it's moved to another system.
  let win_path_max_len = 260;
  let approx_path_prefix_len = 80;
  let max_length = win_path_max_len - approx_path_prefix_len;

  remote_specifiers_to_paths_with_truncation(specifiers, max_length)
}

fn remote_specifiers_to_paths_with_truncation<'a>(
  specifiers: impl Iterator<Item = &'a ModuleSpecifier>,
  max_length: usize,
) -> Vec<(ModuleSpecifier, PathBuf)> {
  #[derive(Default)]
  struct Directory {
    parent: Option<Rc<RefCell<Directory>>>,
    children: Vec<Rc<RefCell<Directory>>>,
    original_name: String,
    name: PathBuf,
    files: Vec<(ModuleSpecifier, PathBuf)>,
    dir_name_set: HashSet<String>,
    file_name_set: HashSet<String>,
  }

  impl Directory {
    pub fn new_root() -> Rc<RefCell<Self>> {
      Rc::new(RefCell::new(Self {
        name: PathBuf::from(""),
        original_name: "".to_string(),
        ..Default::default()
      }))
    }

    pub fn unique_file_name(&mut self, name: &str) -> PathBuf {
      get_unique_path(PathBuf::from(name), &mut self.file_name_set)
    }

    pub fn unique_dir_name(&mut self, name: &str) -> PathBuf {
      get_unique_path(PathBuf::from(name), &mut self.dir_name_set)
    }

    pub fn add_file(&mut self, specifier: ModuleSpecifier, name: &str) {
      let name = self.unique_file_name(name);
      self.files.push((specifier, name));
    }

    pub fn insert_dir_name(
      &mut self,
      index: usize,
      name: &str,
    ) -> Rc<RefCell<Directory>> {
      let new_dir = Rc::new(RefCell::new(Self {
        name: self.unique_dir_name(name),
        original_name: name.to_string(),
        ..Default::default()
      }));
      self.children.insert(index, new_dir.clone());
      new_dir
    }

    pub fn path(&self) -> PathBuf {
      if let Some(parent) = &self.parent {
        parent.borrow().path().join(&self.name)
      } else {
        PathBuf::from(&self.name)
      }
    }

    pub fn path_len(&self) -> usize {
      if self.parent.is_none() && self.name.to_string_lossy().is_empty() {
        0 // root directory has zero length
      } else {
        self.name.to_string_lossy().len()
          + self
            .parent
            .as_ref()
            .map(|p| p.borrow().path_len())
            .unwrap_or(0)
          + 1 // +1 for trailing slash
      }
    }

    pub fn get_or_create_dir(
      dir: &Rc<RefCell<Self>>,
      path: &Path,
    ) -> Rc<RefCell<Self>> {
      let mut current_dir = dir.clone();
      for component in path.components() {
        match component {
          Component::Normal(name) => {
            let name = name.to_string_lossy();
            let new_child = {
              let mut dir = current_dir.borrow_mut();
              // we can only assume it's sorted while building the tree
              match dir.children.binary_search_by(|c| {
                c.borrow().original_name.as_str().cmp(&name)
              }) {
                Ok(index) => dir.children[index].clone(),
                Err(index) => dir.insert_dir_name(index, &name),
              }
            };
            new_child.borrow_mut().parent = Some(current_dir);
            current_dir = new_child;
          }
          _ => panic!("Unexpected path component: {:?}", component),
        }
      }

      current_dir
    }
  }

  // for this code, we assume byte length is equivalent to character length
  // because this is an approximation and we want it to be faster
  let mut dirs_exceeding_length: HashMap<String, Rc<RefCell<Directory>>> =
    HashMap::default();
  let root_dir = Directory::new_root();
  let root_remote_specifiers = partition_by_root_specifiers(specifiers);
  for (root, specifiers) in root_remote_specifiers {
    let base_dir_original_name = dir_name_for_root(&root);
    for specifier in specifiers {
      let file_path =
        base_dir_original_name.join(sanitize_filepath(&specifier.path()[1..]));
      let dir_path = file_path.parent().unwrap().to_owned();

      let dir = Directory::get_or_create_dir(&root_dir, &dir_path);
      let file_name = file_path.file_name().unwrap().to_string_lossy();
      dir.borrow_mut().add_file(specifier.to_owned(), &file_name);

      // this doesn't exactly test the file name length, but that's ok
      if file_path.to_string_lossy().len() > max_length {
        dirs_exceeding_length
          .insert(dir_path.to_string_lossy().to_string(), dir);
      }
    }
  }

  // traverse any directories that exceed the length
  for dir in dirs_exceeding_length.values() {
    let dir_path_len = dir.borrow().path_len();
    let min_filename_chars = 8; // 5 for ext, 3 for filename
    let max_file_path_len =
      std::cmp::max(dir_path_len + min_filename_chars, max_length);
    let max_file_name_len = max_file_path_len - dir_path_len;

    // shorten all the file names
    {
      let mut dir = dir.borrow_mut();
      for i in 0..dir.files.len() {
        let file_name_str = dir.files[i].1.to_string_lossy().to_string();
        if file_name_str.len() > max_file_name_len {
          let new_name =
            if let Some((stem, ext)) = split_stem_and_ext(&file_name_str) {
              let file_name_index = std::cmp::min(
                stem.len(),
                std::cmp::max(
                  3,
                  (max_file_name_len as isize) - (ext.len() as isize + 1),
                ) as usize,
              );
              format!("{}.{}", &stem[..file_name_index], ext)
            } else {
              file_name_str[..max_file_name_len].to_string()
            };
          if new_name != file_name_str {
            dir.files[i].1 = dir.unique_file_name(&new_name);
          }
        }
      }
    }

    let mut difference =
      max_length as isize - (dir_path_len + min_filename_chars) as isize;
    // now check if we need to go up the directories shortening directory names
    let mut next_dir = Some(dir.clone());
    while let Some(dir) = next_dir.take() {
      if difference > 0 {
        break;
      }
      let mut dir = dir.borrow_mut();
      let original_name = dir.name.clone().to_string_lossy().to_string();
      let new_name = &original_name[..std::cmp::min(original_name.len(), 5)];
      if new_name != original_name {
        if let Some(parent) = dir.parent.as_ref().cloned() {
          let mut parent = parent.borrow_mut();
          dir.name = parent.unique_dir_name(new_name);
        } else {
          dir.name = PathBuf::from(new_name);
        }
      }
      difference += original_name.len() as isize
        - dir.name.to_string_lossy().len() as isize
        + 1; // +1 for directory separator
      next_dir = dir.parent.clone();
    }
  }

  let mut result = Vec::new();
  let mut pending_dirs = vec![root_dir];
  while let Some(dir) = pending_dirs.pop() {
    let mut dir = dir.borrow_mut();
    let dir_path = dir.path();
    for (specifier, name) in dir.files.drain(..) {
      result.push((specifier, dir_path.join(name)));
    }
    pending_dirs.extend(dir.children.iter().cloned());
  }

  result
}

fn split_stem_and_ext(path: &str) -> Option<(&str, &str)> {
  let d_ts_ext = ".d.ts";
  if path.to_lowercase().ends_with(d_ts_ext) {
    Some((
      &path[..path.len() - d_ts_ext.len()],
      &path[path.len() - (d_ts_ext.len() - 1)..],
    ))
  } else {
    path
      .rfind('.')
      .map(|index| (&path[..index], &path[index + 1..]))
  }
}

fn get_mapped_file_path(
  media_type: MediaType,
  path: impl AsRef<Path>,
  mapped_filepaths_no_ext: &mut HashSet<String>,
) -> PathBuf {
  fn without_ext(path: impl AsRef<Path>) -> PathBuf {
    // remove the extension if it's known
    // Ex. url could be `https://deno.land/test/1.2.5`
    // and we don't want to use `1.2`
    let media_type: MediaType = path.as_ref().into();
    if media_type == MediaType::Unknown {
      path.as_ref().into()
    } else {
      with_extension(path.as_ref(), "")
    }
  }

  let filepath_no_ext =
    get_unique_path(without_ext(path), mapped_filepaths_no_ext);
  let extension = match media_type {
    MediaType::Json => "js",
    MediaType::Mjs | MediaType::Mts => "js",
    _ => &media_type.as_ts_extension()[1..],
  };
  with_extension(
    &filepath_no_ext,
    &if let Some(sub_ext) = filepath_no_ext.extension() {
      format!("{}.{}", sub_ext.to_string_lossy(), extension)
    } else {
      extension.to_string()
    },
  )
}

/// Gets the directory name to use for the provided root.
fn dir_name_for_root(root: &ModuleSpecifier) -> PathBuf {
  let mut result = String::new();
  if let Some(domain) = root.domain() {
    result.push_str(&sanitize_segment(domain));
  }
  if let Some(port) = root.port() {
    if !result.is_empty() {
      result.push('_');
    }
    result.push_str(&port.to_string());
  }
  let mut result = PathBuf::from(result);
  if let Some(segments) = root.path_segments() {
    for segment in segments.filter(|s| !s.is_empty()) {
      result = result.join(sanitize_segment(segment));
    }
  }

  result
}

fn sanitize_filepath(text: &str) -> String {
  text
    .chars()
    .map(|c| if is_banned_path_char(c) { '_' } else { c })
    .collect()
}

fn is_banned_path_char(c: char) -> bool {
  matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')
}

fn sanitize_segment(text: &str) -> String {
  text
    .chars()
    .map(|c| if is_banned_segment_char(c) { '_' } else { c })
    .collect()
}

fn is_banned_segment_char(c: char) -> bool {
  matches!(c, '/' | '\\') || is_banned_path_char(c)
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
    run_test("http://deno.land/x/test", "deno.land/x/test");
    run_test("http://localhost", "localhost");
    run_test("http://localhost/test%20:test", "localhost/test%20_test");

    fn run_test(specifier: &str, expected: &str) {
      assert_eq!(
        dir_name_for_root(&ModuleSpecifier::parse(specifier).unwrap()),
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

  #[test]
  fn test_remote_specifiers_to_paths() {
    run_remote_specifiers_to_paths_test(
      &[
        "http://localhost/file.json",
        "http://localhost/file.ts",
        "http://localhost/other.ts",
        "http://localhost/other.ts?query",
        "http://localhost/folder/file.json",
        "http://localhost/FOLDER/file.json",
        "http://localhost/other_folder/file.json",
        "http://localhost/sub_folder/file.json",
        "http://localhost/sub_folder/test.json",
        "https://deno.land/x/file.json",
      ],
      &[
        (
          "http://localhost/FOLDER/file.json",
          "localhost/FOLDER_2/file.json",
        ),
        ("http://localhost/file.json", "localhost/file.json"),
        ("http://localhost/file.ts", "localhost/file.ts"),
        (
          "http://localhost/folder/file.json",
          "localhost/folder/file.json",
        ),
        ("http://localhost/other.ts", "localhost/other.ts"),
        ("http://localhost/other.ts?query", "localhost/other_2.ts"),
        (
          "http://localhost/other_folder/file.json",
          "localhost/other_folder/file.json",
        ),
        (
          "http://localhost/sub_folder/file.json",
          "localhost/sub_folder/file.json",
        ),
        (
          "http://localhost/sub_folder/test.json",
          "localhost/sub_folder/test.json",
        ),
        ("https://deno.land/x/file.json", "deno.land/x/file.json"),
      ],
      260,
    )
  }

  #[test]
  fn test_remote_specifiers_to_paths_filename_truncation() {
    run_remote_specifiers_to_paths_test(
      &[
        "http://localhost/1234567890123456.d.ts",
        "http://localhost/1234567890123456.json",
      ],
      &[
        (
          "http://localhost/1234567890123456.d.ts",
          "localhost/1234567890.d.ts",
        ),
        (
          "http://localhost/1234567890123456.json",
          "localhost/1234567890.json",
        ),
      ],
      25,
    );

    run_remote_specifiers_to_paths_test(
      &[
        "http://localhost/1234567890.json",
        "http://localhost/1234567890123456.json",
      ],
      &[
        (
          "http://localhost/1234567890.json",
          "localhost/1234567890.json",
        ),
        (
          "http://localhost/1234567890123456.json",
          "localhost/1234567890_2.json",
        ),
      ],
      25,
    );

    run_remote_specifiers_to_paths_test(
      &["http://localhost/a/b/c/d/e/f/g/h/test.json"],
      &[(
        "http://localhost/a/b/c/d/e/f/g/h/test.json",
        "local/a/b/c/d/e/f/g/h/tes.json",
      )],
      25,
    );

    run_remote_specifiers_to_paths_test(
      &["http://localhost/1234567890123456789/123.5678"],
      &[(
        "http://localhost/1234567890123456789/123.5678",
        "localhost/12345/123.5678",
      )],
      25,
    );
  }

  fn run_remote_specifiers_to_paths_test(
    specifiers: &[&str],
    expected: &[(&str, &str)],
    max_length: usize,
  ) {
    let specifiers = specifiers
      .iter()
      .map(|s| ModuleSpecifier::parse(s).unwrap())
      .collect::<Vec<_>>();
    let result =
      remote_specifiers_to_paths_with_truncation(specifiers.iter(), max_length);
    let result_as_strings = result
      .into_iter()
      .map(|(url, path)| {
        (url.to_string(), path.to_string_lossy().replace('\\', "/"))
      })
      .collect::<Vec<_>>();
    let mut result_as_strs = result_as_strings
      .iter()
      .map(|(u, p)| (u.as_str(), p.as_str()))
      .collect::<Vec<_>>();
    result_as_strs.sort_by_key(|r| r.0);
    assert_eq!(result_as_strs, expected);
  }

  #[test]
  fn test_split_stem_and_ext() {
    assert_eq!(split_stem_and_ext("test.ts"), Some(("test", "ts")));
    assert_eq!(split_stem_and_ext("test.TS"), Some(("test", "TS")));
    assert_eq!(split_stem_and_ext("test.D.TS"), Some(("test", "D.TS")));
    assert_eq!(split_stem_and_ext("test.d.ts"), Some(("test", "d.ts")));
    assert_eq!(
      split_stem_and_ext("test.other.json"),
      Some(("test.other", "json"))
    );
    assert_eq!(split_stem_and_ext("none"), None);
  }
}
