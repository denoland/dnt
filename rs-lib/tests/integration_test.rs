use std::path::PathBuf;

use pretty_assertions::assert_eq;

#[macro_use]
mod integration;

use integration::TestBuilder;

#[tokio::test]
async fn transform_standalone_file() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", r#"test;"#);
    })
    .transform()
    .await
    .unwrap();

  assert_files!(result, &[("mod.ts", "test;")]);
}

#[tokio::test]
async fn transform_deno_shim() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", r#"Deno.readTextFile();"#);
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result,
    &[(
      "mod.ts",
      concat!(
        r#"import * as denoShim from "shim-package-name";"#,
        "\ndenoShim.Deno.readTextFile();"
      )
    )]
  );
}

#[tokio::test]
async fn transform_deno_shim_with_name_collision() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file(
        "/mod.ts",
        r#"Deno.readTextFile(); const denoShim = {};"#,
      );
    })
    .shim_package_name("test-shim")
    .transform()
    .await
    .unwrap();

  assert_files!(
    result,
    &[(
      "mod.ts",
      concat!(
        r#"import * as denoShim1 from "test-shim";"#,
        "\ndenoShim1.Deno.readTextFile(); const denoShim = {};"
      )
    )]
  );
}

#[tokio::test]
async fn transform_global_this_deno() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", r#"globalThis.Deno.readTextFile();"#);
    })
    .shim_package_name("test-shim")
    .transform()
    .await
    .unwrap();

  assert_files!(
    result,
    &[(
      "mod.ts",
      concat!(
        r#"import * as denoShim from "test-shim";"#,
        "\n({ Deno: denoShim.Deno, ...globalThis }).Deno.readTextFile();"
      )
    )]
  );
}

#[tokio::test]
async fn transform_deno_collision() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file(
        "/mod.ts",
        concat!(
          "const Deno = {};",
          "const { Deno: Deno2 } = globalThis;",
          "Deno2.readTextFile();",
          "Deno.test;"
        ),
      );
    })
    .shim_package_name("test-shim")
    .transform()
    .await
    .unwrap();

  assert_files!(
    result,
    &[(
      "mod.ts",
      concat!(
        r#"import * as denoShim from "test-shim";"#,
        "\nconst Deno = {};",
        "const { Deno: Deno2 } = ({ Deno: denoShim.Deno, ...globalThis });",
        "Deno2.readTextFile();",
        "Deno.test;"
      )
    )]
  );
}

#[tokio::test]
async fn transform_other_file_no_extensions() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "import * as other from './other.ts';")
        .add_local_file("/other.ts", "5;");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result,
    &[
      ("mod.ts", "import * as other from './other';"),
      ("other.ts", "5;")
    ]
  );
}

#[tokio::test]
async fn transform_other_file_keep_extensions() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "import * as other from './other.ts';")
        .add_local_file("/other.ts", "5;");
    })
    .keep_extensions()
    .transform()
    .await
    .unwrap();

  assert_files!(
    result,
    &[
      ("mod.ts", "import * as other from './other.js';"),
      ("other.ts", "5;")
    ]
  );
}

#[tokio::test]
async fn transform_remote_files() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          "import * as other from 'http://localhost/mod.ts';",
        )
        .add_remote_file(
          "http://localhost/mod.ts",
          "import * as myOther from './other.ts';",
        )
        .add_remote_file(
          "http://localhost/other.ts",
          "import * as folder from './folder';",
        )
        .add_remote_file(
          "http://localhost/folder",
          "import * as folder2 from './folder.ts';",
        )
        .add_remote_file(
          "http://localhost/folder.ts",
          "import * as folder3 from './folder.js';",
        )
        .add_remote_file(
          "http://localhost/folder.js",
          "import * as otherFolder from './otherFolder';",
        )
        .add_remote_file(
          "http://localhost/otherFolder",
          "import * as subFolder from './sub/subfolder';",
        )
        .add_remote_file(
          "http://localhost/sub/subfolder",
          "import * as localhost2 from 'http://localhost2';",
        )
        .add_remote_file(
          "http://localhost2",
          "import * as localhost3Mod from 'https://localhost3/mod.ts';",
        )
        .add_remote_file(
          "https://localhost3/mod.ts",
          "import * as localhost3 from 'https://localhost3';",
        )
        .add_remote_file_with_headers(
          "https://localhost3",
          "5;",
          &[("content-type", "application/typescript; charset=UTF-8")],
        );
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result,
    &[
      ("mod.ts", "import * as other from './deps/0/mod';"),
      ("deps/0/mod.ts", "import * as myOther from './other';"),
      ("deps/0/other.ts", "import * as folder from './folder';"),
      ("deps/0/folder.js", "import * as folder2 from './folder_2';"),
      (
        "deps/0/folder_2.ts",
        "import * as folder3 from './folder_3';"
      ),
      (
        "deps/0/folder_3.js",
        "import * as otherFolder from './otherFolder';"
      ),
      (
        "deps/0/otherFolder.js",
        "import * as subFolder from './sub/subfolder';"
      ),
      (
        "deps/0/sub/subfolder.js",
        "import * as localhost2 from '../../1';"
      ),
      ("deps/1.js", "import * as localhost3Mod from './2/mod';"),
      ("deps/2/mod.ts", "import * as localhost3 from '../2';"),
      ("deps/2.ts", "5;"),
    ]
  );
}

#[tokio::test]
async fn transform_local_file_not_exists() {
  let err_message = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", "import * as other from './other.ts';");
    })
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(err_message.to_string(), "An error was returned from the loader: entity not found (file:///other.ts)");
}

#[tokio::test]
async fn transform_remote_file_not_exists() {
  let err_message = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_remote_file(
        "http://localhost/mod.ts",
        "import * as other from './other.ts';",
      );
    })
    .entry_point("http://localhost/mod.ts")
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(err_message.to_string(), "An error was returned from the loader: Not found. (http://localhost/other.ts)");
}

#[tokio::test]
async fn transform_remote_file_error() {
  let err_message = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_remote_file_with_error(
        "http://localhost/mod.ts",
        "Some error loading.",
      );
    })
    .entry_point("http://localhost/mod.ts")
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(err_message.to_string(), "An error was returned from the loader: Some error loading. (http://localhost/mod.ts)");
}

#[tokio::test]
async fn transform_parse_error() {
  let err_message = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "export * from 'http://localhost/mod.js';")
        .add_remote_file_with_headers(
          "http://localhost/mod.js",
          "",
          &[("x-typescript-types", "./declarations.d.ts")],
        )
        .add_remote_file(
          "http://localhost/declarations.d.ts",
          "test test test",
        );
    })
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(err_message.to_string(), "The module's source code would not be parsed: Expected ';', '}' or <eof> at http://localhost/declarations.d.ts:1:6 (http://localhost/declarations.d.ts)");
}

#[tokio::test]
async fn transform_typescript_types_resolution_error() {
  let err_message = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "export * from 'https://localhost/mod.js';")
        .add_remote_file_with_headers(
          "https://localhost/mod.js",
          "",
          &[("x-typescript-types", "http://localhost/declarations.d.ts")],
        )
        .add_remote_file("http://localhost/declarations.d.ts", "");
    })
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(err_message.to_string(), "Error resolving types for https://localhost/mod.js with reference http://localhost/declarations.d.ts. Modules imported via https are not allowed to import http modules.");
}

#[tokio::test]
async fn transform_typescript_types_in_headers() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "export * from 'http://localhost/mod.js';")
        .add_remote_file_with_headers(
          "http://localhost/mod.js",
          "function test() { return 5; }",
          &[("x-typescript-types", "./declarations.d.ts")],
        )
        .add_remote_file(
          "http://localhost/declarations.d.ts",
          "declare function test(): number;",
        );
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result,
    &[
      ("mod.ts", "export * from './deps/0/mod';"),
      ("deps/0/mod.js", "function test() { return 5; }"),
      ("deps/0/mod.d.ts", "declare function test(): number;"),
    ]
  );
}

#[ignore]
#[tokio::test]
async fn transform_typescript_types_in_deno_types() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", "// @deno-types='./declarations.d.ts';\nexport * from 'http://localhost/mod.js';")
      .add_remote_file("http://localhost/mod.js", "function test() { return 5; }")
      .add_local_file("/declarations.d.ts", "declare function test(): number;");
    })
    .transform().await.unwrap();

  assert_files!(
    result,
    &[
      // todo: remove this deno-types comment
      (
        "mod.ts",
        "// @deno-types='./declarations.d.ts';\nexport * from './deps/0/mod';"
      ),
      ("deps/0/mod.js", "function test() { return 5; }"),
      ("deps/0/mod.d.ts", "declare function test(): number;"),
    ]
  );
}

#[tokio::test]
async fn transform_typescript_type_references() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", "export * from 'http://localhost/mod.js';")
      .add_remote_file("http://localhost/mod.js", "/// <reference types='./declarations.d.ts' />\nfunction test() { return 5; }")
      .add_remote_file("http://localhost/declarations.d.ts", "declare function test(): number;");
    })
    .transform().await.unwrap();

  assert_files!(result, &[
    ("mod.ts", "export * from './deps/0/mod';"),
    // todo: remove this type reference directive comment
    ("deps/0/mod.js", "/// <reference types='./declarations.d.ts' />\nfunction test() { return 5; }"),
    ("deps/0/mod.d.ts", "declare function test(): number;"),
  ]);
}
