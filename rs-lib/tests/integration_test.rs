// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;

use deno_node_transform::Dependency;
use pretty_assertions::assert_eq;

#[macro_use]
mod integration;

use integration::TestBuilder;

use crate::integration::assert_identity_transforms;
use crate::integration::assert_transforms;

#[tokio::test]
async fn transform_standalone_file() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", r#"test;"#);
    })
    .transform()
    .await
    .unwrap();

  assert_files!(result.main.files, &[("mod.ts", "test;")]);
}

#[tokio::test]
async fn transform_deno_shim() {
  assert_transforms(vec![
    (
      "Deno.readTextFile();",
      concat!(
        r#"import * as denoShim from "test-shim";"#,
        "\ndenoShim.Deno.readTextFile();"
      ),
    ),
    (
      "const [test=Deno] = other;",
      concat!(
        r#"import * as denoShim from "test-shim";"#,
        "\nconst [test=denoShim.Deno] = other;"
      ),
    ),
    (
      "const obj = { test: Deno };",
      concat!(
        r#"import * as denoShim from "test-shim";"#,
        "\nconst obj = { test: denoShim.Deno };"
      ),
    ),
    (
      concat!(
        "const decl01 = Blob;\n",
        "const decl02 = crypto;\n",
        "const decl03 = fetch;\n",
        "const decl04 = File;\n",
        "const decl05 = FormData;\n",
        "const decl06 = Headers;\n",
        "const decl07 = Request;\n",
        "const decl08 = Response;\n",
        "const decl09 = alert;\n",
        "const decl10 = confirm;\n",
        "const decl11: typeof prompt = prompt;\n",
        "setTimeout(() => {}, 100);\n",
        "setInterval(() => {}, 100);\n",
      ),
      concat!(
        r#"import * as denoShim from "test-shim";"#,
        "\nconst decl01 = denoShim.Blob;\n",
        "const decl02 = denoShim.crypto;\n",
        "const decl03 = denoShim.fetch;\n",
        "const decl04 = denoShim.File;\n",
        "const decl05 = denoShim.FormData;\n",
        "const decl06 = denoShim.Headers;\n",
        "const decl07 = denoShim.Request;\n",
        "const decl08 = denoShim.Response;\n",
        "const decl09 = denoShim.alert;\n",
        "const decl10 = denoShim.confirm;\n",
        "const decl11: typeof denoShim.prompt = denoShim.prompt;\n",
        "denoShim.setTimeout(() => {}, 100);\n",
        "denoShim.setInterval(() => {}, 100);\n",
      ),
    ),
  ])
  .await;
}

#[tokio::test]
async fn no_transform_deno_ignored() {
  assert_identity_transforms(vec!["// deno-shim-ignore\nDeno.readTextFile();"])
    .await;
}

#[tokio::test]
async fn transform_deno_shim_with_name_collision() {
  assert_transforms(vec![(
    "Deno.readTextFile(); const denoShim = {};",
    concat!(
      r#"import * as denoShim1 from "test-shim";"#,
      "\ndenoShim1.Deno.readTextFile(); const denoShim = {};"
    ),
  )])
  .await;
}

#[tokio::test]
async fn transform_global_this_deno() {
  assert_transforms(vec![
    (
      concat!(
        "globalThis.Deno.readTextFile();",
        "globalThis.test();",
        "globalThis.test.test();",
        "globalThis['test']();",
        r#"globalThis["test"]();"#,
        "globalThis.Deno = 5;",
        "true ? globalThis : globalThis;",
        "typeof globalThis.Deno;",
      ),
      concat!(
        r#"import * as denoShim from "test-shim";"#,
        "\n({ ...denoShim, ...globalThis }).Deno.readTextFile();",
        "globalThis.test();",
        "globalThis.test.test();",
        "globalThis['test']();",
        r#"globalThis["test"]();"#,
        "globalThis.Deno = 5;",
        "true ? ({ ...denoShim, ...globalThis }) : ({ ...denoShim, ...globalThis });",
        "typeof ({ ...denoShim, ...globalThis }).Deno;"
      )
    ),
  ]).await;
}

#[tokio::test]
async fn no_shim_situations() {
  assert_identity_transforms(vec![
    "const { Deno } = test;",
    "const { asdf, ...Deno } = test;",
    "const { Deno: test } = test;",
    "const { test: Deno } = test;",
    "const [Deno] = test;",
    "const [test, ...Deno] = test;",
    "const obj = { Deno: test };",
    "interface Deno {}",
    "interface Test { Deno: string; }",
    "interface Test { Deno(): string; }",
    "class Deno {}",
    "class Test { Deno: string; }",
    "class Test { Deno() {} }",
    "const t = class Deno {};",
    "function Deno() {}",
    "const t = function Deno() {};",
    "import { Deno } from 'test';",
    "import * as Deno from 'test';",
    "import { test as Deno } from 'test';",
    "import { Deno as test } from 'test';",
    "export { Deno } from 'test';",
    "export * as Deno from 'test';",
    "export { test as Deno } from 'test';",
    "export { Deno as test } from 'test';",
    "try {} catch (Deno) {}",
    "function test(Deno) {}",
    "typeof globalThis;",
    "globalThis == null;",
    "globalThis ? true : false;",
    "type Test = typeof globalThis;",
  ])
  .await;
}

#[tokio::test]
async fn transform_deno_collision() {
  assert_transforms(vec![(
    concat!(
      "const Deno = {};",
      "const { Deno: Deno2 } = globalThis;",
      "Deno2.readTextFile();",
      "Deno.test;"
    ),
    concat!(
      r#"import * as denoShim from "test-shim";"#,
      "\nconst Deno = {};",
      "const { Deno: Deno2 } = ({ ...denoShim, ...globalThis });",
      "Deno2.readTextFile();",
      "Deno.test;"
    ),
  )])
  .await;
}

#[tokio::test]
async fn transform_relative_file() {
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
    result.main.files,
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
          concat!(
            "import * as other from 'http://localhost/mod.ts';\n",
            "import 'https://deno.land/std@0.102.0/mod.ts';",
          ),
        )
        .add_remote_file(
          "http://localhost/mod.ts",
          "import * as myOther from './other.ts';",
        )
        .add_remote_file(
          "http://localhost/other.ts",
          "import * as folder from './folder';",
        )
        .add_remote_file_with_headers(
          "http://localhost/folder",
          "import * as folder2 from './folder.ts';",
          &[("content-type", "application/javascript")],
        )
        .add_remote_file(
          "http://localhost/folder.ts",
          "import * as folder3 from './folder.js';",
        )
        .add_remote_file(
          "http://localhost/folder.js",
          "import * as otherFolder from './otherFolder';",
        )
        .add_remote_file_with_headers(
          "http://localhost/otherFolder",
          "import * as subFolder from './sub/subfolder';",
          &[("content-type", "application/javascript")],
        )
        .add_remote_file_with_headers(
          "http://localhost/sub/subfolder",
          "import * as localhost2 from 'http://localhost2';",
          &[("content-type", "application/javascript")],
        )
        .add_remote_file(
          "https://deno.land/std@0.102.0/mod.ts",
          "console.log(5);",
        )
        .add_remote_file_with_headers(
          "http://localhost2",
          "import * as localhost3Mod from 'https://localhost3/mod.ts';",
          &[("content-type", "application/javascript")],
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
    result.main.files,
    &[
      (
        "mod.ts",
        concat!(
          "import * as other from './deps/localhost/mod.js';\n",
          "import './deps/deno_land/std_0.102.0/mod.js';",
        )
      ),
      (
        "deps/localhost/mod.ts",
        "import * as myOther from './other.js';"
      ),
      (
        "deps/localhost/other.ts",
        "import * as folder from './folder.js';"
      ),
      (
        "deps/localhost/folder.js",
        "import * as folder2 from './folder_2.js';"
      ),
      (
        "deps/localhost/folder_2.ts",
        "import * as folder3 from './folder_3.js';"
      ),
      (
        "deps/localhost/folder_3.js",
        "import * as otherFolder from './otherFolder.js';"
      ),
      (
        "deps/localhost/otherFolder.js",
        "import * as subFolder from './sub/subfolder.js';"
      ),
      (
        "deps/localhost/sub/subfolder.js",
        "import * as localhost2 from '../../localhost2.js';"
      ),
      ("deps/deno_land/std_0.102.0/mod.ts", "console.log(5);"),
      (
        "deps/localhost2.js",
        "import * as localhost3Mod from './localhost3/mod.js';"
      ),
      (
        "deps/localhost3/mod.ts",
        "import * as localhost3 from '../localhost3.js';"
      ),
      ("deps/localhost3.ts", "5;"),
    ]
  );
}

#[tokio::test]
async fn transform_handle_local_deps_folder() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          "import 'http://localhost/mod.ts';\nimport './deps/localhost/mod.ts'",
        )
        .add_local_file("/deps/localhost/mod.ts", "local;")
        .add_remote_file("http://localhost/mod.ts", "remote;");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      (
        "mod.ts",
        "import './deps_2/localhost/mod.js';\nimport './deps/localhost/mod.js'"
      ),
      ("deps/localhost/mod.ts", "local;"),
      ("deps_2/localhost/mod.ts", "remote;"),
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

  assert_eq!(err_message.to_string(), "file not found (file:///other.ts)");
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

  assert_eq!(
    err_message.to_string(),
    "Not found. (http://localhost/other.ts)"
  );
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

  assert_eq!(
    err_message.to_string(),
    "Some error loading. (http://localhost/mod.ts)"
  );
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

  assert_eq!(err_message.to_string(), "The module's source code could not be parsed: Expected ';', '}' or <eof> at http://localhost/declarations.d.ts:1:6 (http://localhost/declarations.d.ts)");
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

  assert_eq!(err_message.to_string(),
    concat!(
      "Error resolving types for https://localhost/mod.js with reference http://localhost/declarations.d.ts. ",
      "Modules imported via https are not allowed to import http modules.\n",
      "  Importing: http://localhost/declarations.d.ts"
    )
  );
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
    result.main.files,
    &[
      ("mod.ts", "export * from './deps/localhost/mod.js';"),
      ("deps/localhost/mod.js", "function test() { return 5; }"),
      (
        "deps/localhost/mod.d.ts",
        "declare function test(): number;"
      ),
    ]
  );
}

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
    result.main.files,
    &[
      ("mod.ts", "export * from './deps/localhost/mod.js';"),
      ("deps/localhost/mod.js", "function test() { return 5; }"),
      (
        "deps/localhost/mod.d.ts",
        "declare function test(): number;"
      ),
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

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", "export * from './deps/localhost/mod.js';"),
      ("deps/localhost/mod.js", "function test() { return 5; }"),
      (
        "deps/localhost/mod.d.ts",
        "declare function test(): number;"
      ),
    ]
  );
}

#[tokio::test]
async fn transform_deno_types_and_type_ref_for_same_file() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", "// @deno-types='./declarations.d.ts'\nexport * from './file.js';\n// @deno-types='./declarations.d.ts'\nexport * as test2 from './file.js';\nexport * from './other.ts';")
      .add_local_file("/file.js", "/// <reference types='./declarations.d.ts' />\nfunction test() { return 5; }")
      .add_local_file("/other.ts", "// @deno-types='./declarations.d.ts'\nexport * as other from './file.js';")
      .add_local_file("/declarations.d.ts", "declare function test(): number;");
    })
    .transform().await.unwrap();

  assert!(result.warnings.is_empty());
  assert_files!(
    result.main.files,
    &[
      (
        "mod.ts",
        "export * from './file.js';\nexport * as test2 from './file.js';\nexport * from './other.js';"
      ),
      (
        "other.ts",
        "export * as other from './file.js';"
      ),
      ("file.js", "function test() { return 5; }"),
      ("file.d.ts", "declare function test(): number;"),
    ]
  );
}

#[tokio::test]
async fn transform_deno_types_and_type_ref_for_different_local_file() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file(
        "/mod.ts",
        "// @deno-types='./declarations.d.ts'\nexport * from './file.js';\nexport * from './other.ts';"
      )
      .add_local_file("/file.js", "/// <reference types='./declarations3.d.ts' />\nfunction test() { return 5; }")
      .add_local_file("/other.ts", "// @deno-types='./declarations2.d.ts'\nexport * as other from './file.js';")
      .add_local_file("/declarations.d.ts", "declare function test1(): number;")
      .add_local_file("/declarations2.d.ts", "declare function test2(): number;")
      .add_local_file("/declarations3.d.ts", "declare function test3(): number;");
    })
    .transform().await.unwrap();

  assert_eq!(
    result.warnings,
    vec![
      concat!(
        "Duplicate declaration file found for file:///file.js\n",
        "  Specified file:///declarations.d.ts in file:///mod.ts\n",
        "  Selected file:///declarations3.d.ts\n",
        "  Supress this warning by having only one local file specify the declaration file for this module.",
      ),
      concat!(
        "Duplicate declaration file found for file:///file.js\n",
        "  Specified file:///declarations2.d.ts in file:///other.ts\n",
        "  Selected file:///declarations3.d.ts\n",
        "  Supress this warning by having only one local file specify the declaration file for this module.",
      ),
    ]
  );
  assert_files!(
    result.main.files,
    &[
      (
        "mod.ts",
        "export * from './file.js';\nexport * from './other.js';"
      ),
      ("other.ts", "export * as other from './file.js';"),
      ("file.js", "function test() { return 5; }"),
      ("file.d.ts", "declare function test3(): number;"),
    ]
  );
}

#[tokio::test]
async fn transform_deno_types_and_type_ref_for_different_remote_file() {
  fn setup() -> TestBuilder {
    let mut test_builder = TestBuilder::new();
    test_builder .with_loader(|loader| {
        loader.add_local_file(
          "/mod.ts",
          "import 'http://localhost/mod.ts';"
        )
        .add_remote_file(
          "http://localhost/mod.ts",
          "// @deno-types='./declarations.d.ts'\nexport * from './file.js';\nexport * from './other.ts';"
        )
        .add_remote_file("http://localhost/file.js", "/// <reference types='./declarations3.d.ts' />\nfunction test() { return 5; }")
        .add_remote_file("http://localhost/other.ts", "// @deno-types='./declarations2.d.ts'\nexport * as other from './file.js';")
        .add_remote_file("http://localhost/declarations.d.ts", "declare function test1(): number;")
        .add_remote_file("http://localhost/declarations2.d.ts", "declare function test2(): number;")
        .add_remote_file("http://localhost/declarations3.d.ts", "declare function test3(): number;");
      });
    test_builder
  }

  let result = setup().transform().await.unwrap();

  assert_eq!(
    result.warnings,
    vec![
      concat!(
        "Duplicate declaration file found for http://localhost/file.js\n",
        "  Specified http://localhost/declarations.d.ts in http://localhost/mod.ts\n",
        "  Selected http://localhost/declarations3.d.ts\n",
        "  Supress this warning by specifying a declaration file for this module locally via `@deno-types`.",
      ),
      concat!(
        "Duplicate declaration file found for http://localhost/file.js\n",
        "  Specified http://localhost/declarations2.d.ts in http://localhost/other.ts\n",
        "  Selected http://localhost/declarations3.d.ts\n",
        "  Supress this warning by specifying a declaration file for this module locally via `@deno-types`.",
      ),
    ]
  );
  assert_files!(
    result.main.files,
    &[
      ("mod.ts", "import './deps/localhost/mod.js';",),
      (
        "deps/localhost/mod.ts",
        "export * from './file.js';\nexport * from './other.js';"
      ),
      (
        "deps/localhost/other.ts",
        "export * as other from './file.js';"
      ),
      ("deps/localhost/file.js", "function test() { return 5; }"),
      (
        "deps/localhost/file.d.ts",
        "declare function test3(): number;"
      ),
    ]
  );

  // Now specify the declaration file locally. This should clear out the warnings.
  let mut test_builder = setup();
  test_builder.with_loader(|loader| {
    // overwrite the existing /mod.ts
    loader.add_local_file(
      "/mod.ts",
      "import 'http://localhost/mod.ts';\n// @deno-types='http://localhost/declarations2.d.ts'\nimport * as test from 'http://localhost/file.js'",
    );
  });
  let result = test_builder.transform().await.unwrap();

  assert!(result.warnings.is_empty());
  assert_eq!(result.main.files.len(), 5);
  assert_eq!(
    result
      .main
      .files
      .iter()
      .find(|f| f.file_path == PathBuf::from("deps/localhost/file.d.ts"))
      .unwrap()
      .file_text,
    "declare function test2(): number;"
  );
}

#[tokio::test]
async fn transform_specifier_mappings() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          concat!(
            "import * as remote from 'http://localhost/mod.ts';\n",
            "import * as local from './file.ts';\n"
          ),
        )
        .add_remote_file(
          "http://localhost/mod.ts",
          "import * as myOther from './other.ts';",
        );
    })
    .add_specifier_mapping(
      "http://localhost/mod.ts",
      "remote-module",
      Some("1.0.0"),
    )
    .add_specifier_mapping("file:///file.ts", "local-module", None)
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", "import * as remote from 'remote-module';\nimport * as local from 'local-module';\n"),
    ]
  );
  assert_eq!(
    result.main.dependencies,
    &[Dependency {
      name: "remote-module".to_string(),
      version: "1.0.0".to_string(),
    },]
  );
}

#[tokio::test]
async fn transform_not_found_mappings() {
  let error_message = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", "test");
    })
    .add_specifier_mapping("http://localhost/mod.ts", "local-module", None)
    .add_specifier_mapping("http://localhost/mod2.ts", "local-module2", None)
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(
    error_message.to_string(),
    "The following specifiers were indicated to be mapped, but were not found:\n  * http://localhost/mod.ts\n  * http://localhost/mod2.ts"
  );
}

#[tokio::test]
async fn node_module_mapping() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          concat!(
            "import * as path from 'https://deno.land/std@0.109.0/node/path.ts';\n",
            "import * as fs from 'https://deno.land/std/node/fs/promises.ts';",
          ),
        );
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[(
      "mod.ts",
      concat!(
        "import * as path from 'path';\n",
        "import * as fs from 'fs/promises';",
      )
    ),]
  );
}

#[tokio::test]
async fn skypack_esm_module_mapping() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          concat!(
            "import package1 from 'https://cdn.skypack.dev/preact@^10.5.0';\n",
            "import package2 from 'https://cdn.skypack.dev/@scope/package-name@1';\n",
            "import package3 from 'https://esm.sh/react@17.0.2';\n",
            // custom esm.sh stuff like this should download the dependency
            "import package4 from 'https://esm.sh/swr?deps=react@16.14.0';\n",
            "import package5 from 'https://esm.sh/test@1.2.5?deps=react@16.14.0';\n",
          ),
        )
        .add_remote_file_with_headers(
          "https://esm.sh/swr?deps=react@16.14.0", "",
          &[("content-type", "application/typescript")]
        )
        .add_remote_file_with_headers(
          "https://esm.sh/test@1.2.5?deps=react@16.14.0",
          "",
          &[("content-type", "application/typescript")]
       );
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      (
        "mod.ts",
        concat!(
          "import package1 from 'preact';\n",
          "import package2 from '@scope/package-name';\n",
          "import package3 from 'react';\n",
          "import package4 from './deps/esm_sh/swr.js';\n",
          "import package5 from './deps/esm_sh/test_1.js';\n"
        )
      ),
      ("deps/esm_sh/swr.ts", "",),
      ("deps/esm_sh/test_1.ts", "",)
    ]
  );
  assert_eq!(
    result.main.dependencies,
    &[
      Dependency {
        name: "@scope/package-name".to_string(),
        version: "1".to_string(),
      },
      Dependency {
        name: "preact".to_string(),
        version: "^10.5.0".to_string(),
      },
      Dependency {
        name: "react".to_string(),
        version: "17.0.2".to_string(),
      },
    ]
  );
}

#[tokio::test]
async fn skypack_module_mapping_different_versions() {
  let error_message = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file(
        "/mod.ts",
        concat!(
          "import package1 from 'https://cdn.skypack.dev/preact@^10.5.0';\n",
          "import package2 from 'https://cdn.skypack.dev/preact@^10.5.2';",
        ),
      );
    })
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(
    error_message.to_string(),
    "Specifier https://cdn.skypack.dev/preact@^10.5.0 with version ^10.5.0 did not match specifier https://cdn.skypack.dev/preact@^10.5.2 with version ^10.5.2."
  );
}

#[tokio::test]
async fn transform_multiple_entry_points() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "import './ref.ts';mod1;")
        .add_local_file("/mod2.ts", "import './ref.ts';mod2;")
        .add_local_file("/ref.ts", "export const test = 5;");
    })
    .add_entry_point("file:///mod2.ts")
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", "import './ref.js';mod1;"),
      ("mod2.ts", "import './ref.js';mod2;"),
      ("ref.ts", "export const test = 5;"),
    ]
  );
}

#[tokio::test]
async fn test_entry_points() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          "import package1 from 'https://cdn.skypack.dev/preact@^10.5.0';\n",
        )
        .add_local_file(
          "/mod.test.ts",
          concat!(
            "import './mod.ts';\n",
            "import package1 from 'https://cdn.skypack.dev/preact@^10.5.0';\n",
            "import package3 from 'https://esm.sh/react@17.0.2';\n",
            "Deno.writeTextFile('test', 'test')",
          ),
        );
    })
    .add_test_entry_point("file:///mod.test.ts")
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[("mod.ts", "import package1 from 'preact';\n",)]
  );
  assert_eq!(
    result.main.dependencies,
    &[Dependency {
      name: "preact".to_string(),
      version: "^10.5.0".to_string(),
    },]
  );
  assert_eq!(result.main.entry_points, &["mod.ts"]);
  assert_eq!(result.main.shim_used, false);

  assert_files!(
    result.test.files,
    &[(
      "mod.test.ts",
      concat!(
        "import * as denoShim from \"deno.ns\";\n",
        "import './mod.js';\n",
        "import package1 from 'preact';\n",
        "import package3 from 'react';\n",
        "denoShim.Deno.writeTextFile('test', 'test')"
      ),
    )]
  );
  assert_eq!(
    result.test.dependencies,
    &[Dependency {
      name: "react".to_string(),
      version: "17.0.2".to_string(),
    },]
  );
  assert_eq!(result.test.entry_points, &["mod.test.ts"]);
  assert_eq!(result.test.shim_used, true);
}

#[tokio::test]
async fn test_entry_points_same_module_multiple_places() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          concat!(
            "export * from 'https://deno.land/std@0.102.0/path.ts';\n",
            "import * as deps from './deps.ts';",
          ),
        )
        // ensure that the path.ts in this file being already analyzed
        // doesn't cause flags.ts to not be analyzed
        .add_local_file(
          "/deps.ts",
          concat!(
            "export * from 'https://deno.land/std@0.102.0/path.ts';\n",
            "export * from 'https://deno.land/std@0.102.0/flags.ts';",
          ),
        )
        .add_remote_file(
          "https://deno.land/std@0.102.0/flags.ts",
          "export class Flags {}",
        )
        .add_remote_file(
          "https://deno.land/std@0.102.0/path.ts",
          "export class Path {}",
        )
        .add_local_file("/mod.test.ts", "import * as deps from './deps.ts';");
    })
    .add_test_entry_point("file:///mod.test.ts")
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      (
        "mod.ts",
        concat!(
          "export * from './deps/deno_land/std_0.102.0/path.js';\n",
          "import * as deps from './deps.js';",
        )
      ),
      (
        "deps.ts",
        concat!(
          "export * from './deps/deno_land/std_0.102.0/path.js';\n",
          "export * from './deps/deno_land/std_0.102.0/flags.js';",
        )
      ),
      (
        "deps/deno_land/std_0.102.0/flags.ts",
        "export class Flags {}"
      ),
      ("deps/deno_land/std_0.102.0/path.ts", "export class Path {}")
    ]
  );
  assert_eq!(result.main.entry_points, &["mod.ts"]);
  assert_eq!(result.main.shim_used, false);

  assert_files!(
    result.test.files,
    &[("mod.test.ts", "import * as deps from './deps.js';",)]
  );
  assert_eq!(result.test.entry_points, &["mod.test.ts"]);
  assert_eq!(result.test.shim_used, false);
}
