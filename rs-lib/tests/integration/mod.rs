// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

mod in_memory_loader;
mod test_builder;

pub use in_memory_loader::*;
pub use test_builder::*;

macro_rules! assert_files {
  ($actual: expr, $expected: expr) => {{
    let mut actual = $actual;
    let expected = $expected;
    #[cfg(target_os = "windows")]
    for file in actual.iter_mut() {
      // normalize this on windows to forward slashes
      file.file_path = std::path::PathBuf::from(
        file
          .file_path
          .to_string_lossy()
          .to_string()
          .replace("\\", "/"),
      );
    }
    actual.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    let mut expected = expected
      .iter()
      .map(|(file_path, file_text)| deno_node_transform::OutputFile {
        file_path: std::path::PathBuf::from(file_path),
        file_text: file_text.to_string(),
      })
      .collect::<Vec<_>>();
    expected.sort_by(|a, b| a.file_path.cmp(&b.file_path));

    pretty_assertions::assert_eq!(actual, expected);
  }};
}

pub async fn assert_transforms(files: Vec<(&str, &str)>) {
  let files = files
    .into_iter()
    .enumerate()
    .map(|(i, file)| {
      (
        format!(
          "mod{}.ts",
          if i == 0 {
            "".to_string()
          } else {
            i.to_string()
          }
        ),
        file,
      )
    })
    .collect::<Vec<_>>();
  let mut test_builder = TestBuilder::new();
  test_builder
    .with_loader(|loader| {
      for (file_name, file) in files.iter() {
        loader.add_local_file(&format!("/{}", file_name), file.0);
      }
    })
    .add_default_shims();

  for i in 1..files.len() {
    test_builder.add_entry_point(format!("file:///mod{}.ts", i));
  }

  let result = test_builder.transform().await.unwrap();
  let expected_files = files
    .into_iter()
    .map(|(file_name, file)| (file_name, file.1))
    .collect::<Vec<_>>();
  let actual_files = result
    .main
    .files
    .into_iter()
    .filter(|f| !f.file_path.ends_with("_dnt.shims.ts"))
    .collect::<Vec<_>>();
  assert_files!(actual_files, expected_files);
}

pub async fn assert_identity_transforms(files: Vec<&str>) {
  assert_transforms(files.into_iter().map(|text| (text, text)).collect()).await
}
