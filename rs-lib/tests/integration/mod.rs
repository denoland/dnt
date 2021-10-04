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
      file.file_path = PathBuf::from(
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
        file_path: PathBuf::from(file_path),
        file_text: file_text.to_string(),
      })
      .collect::<Vec<_>>();
    expected.sort_by(|a, b| a.file_path.cmp(&b.file_path));

    pretty_assertions::assert_eq!(actual, expected);
  }};
}
