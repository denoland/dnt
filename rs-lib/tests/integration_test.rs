// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;

use deno_node_transform::Dependency;
use deno_node_transform::GlobalName;
use deno_node_transform::ModuleShim;
use deno_node_transform::PackageMappedSpecifier;
use deno_node_transform::PackageShim;
use deno_node_transform::ScriptTarget;
use deno_node_transform::Shim;
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
async fn transform_shims() {
  assert_transforms(vec![
    (
      "Deno.readTextFile();",
      concat!(
        r#"import * as dntShim from "./_dnt.shims.js";"#,
        "\ndntShim.Deno.readTextFile();"
      ),
    ),
    (
      concat!("// copyright comment\n", "Deno.readTextFile();",),
      // should inject after the copyright comment
      concat!(
        "// copyright comment\n",
        r#"import * as dntShim from "./_dnt.shims.js";"#,
        "\n\ndntShim.Deno.readTextFile();"
      ),
    ),
    (
      concat!("// @ts-ignore\n", "Deno.readTextFile();",),
      // should inject before the non-copyright comment
      concat!(
        r#"import * as dntShim from "./_dnt.shims.js";"#,
        "\n// @ts-ignore\n",
        "dntShim.Deno.readTextFile();"
      ),
    ),
    (
      "const [test=Deno] = other;",
      concat!(
        r#"import * as dntShim from "./_dnt.shims.js";"#,
        "\nconst [test=dntShim.Deno] = other;"
      ),
    ),
    (
      "const obj = { test: Deno };",
      concat!(
        r#"import * as dntShim from "./_dnt.shims.js";"#,
        "\nconst obj = { test: dntShim.Deno };"
      ),
    ),
    (
      concat!(
        "const decl01 = Deno;\n",
        "const decl02 = setTimeout;\n",
        "const decl03 = setInterval;\n",
        "const decl04: typeof setTimeout = setTimeout;\n",
        "setTimeout(() => {}, 100);\n",
        "if ('test' in Deno) {}\n",
      ),
      concat!(
        r#"import * as dntShim from "./_dnt.shims.js";"#,
        "\nconst decl01 = dntShim.Deno;\n",
        "const decl02 = dntShim.setTimeout;\n",
        "const decl03 = dntShim.setInterval;\n",
        "const decl04: typeof dntShim.setTimeout = dntShim.setTimeout;\n",
        "dntShim.setTimeout(() => {}, 100);\n",
        "if ('test' in dntShim.Deno) {}\n",
      ),
    ),
    (
      // previously this would panic
      "// @deno-types='test'\nDeno.readTextFile();",
      concat!(
        "\n",
        r#"import * as dntShim from "./_dnt.shims.js";"#,
        "\n\ndntShim.Deno.readTextFile();"
      ),
    ),
  ])
  .await;
}

#[tokio::test]
async fn transform_shim_custom_shims() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file(
        "/mod.ts",
        "fetch(); console.log(Blob); fetchTest(); DOMException; BareModule; LocalShim;",
      ).add_local_file(
        "/local_shim.ts",
        "export class LocalShim {} fetch()",
      ).add_remote_file(
        "https://localhost/remote_shim.ts",
        "export class RemoteShim {} fetch()",
      );
    })
    .add_shim(Shim::Package(PackageShim {
      package: PackageMappedSpecifier {
        name: "node-fetch".to_string(),
        version: Some("~3.1.0".to_string()),
        sub_path: None,
        peer_dependency: false,
      },
      types_package: None,
      global_names: vec![GlobalName {
        name: "fetch".to_string(),
        export_name: Some("default".to_string()),
        type_only: false,
      }],
    }))
    .add_shim(Shim::Package(PackageShim {
      package: PackageMappedSpecifier {
        name: "node-fetch".to_string(),
        version: Some("~3.1.0".to_string()),
        sub_path: Some("test".to_string()),
        peer_dependency: false,
      },
      types_package: None,
      global_names: vec![GlobalName {
        name: "fetchTest".to_string(),
        export_name: Some("fetchTestName".to_string()),
        type_only: false,
      }],
    }))
    .add_shim(Shim::Package(PackageShim {
      package: PackageMappedSpecifier {
        name: "domexception".to_string(),
        version: Some("^4.0.0".to_string()),
        sub_path: None,
        peer_dependency: false,
      },
      types_package: Some(Dependency {
        name: "@types/domexception".to_string(),
        version: "^2.0.1".to_string(),
        peer_dependency: false,
      }),
      global_names: vec![GlobalName {
        name: "DOMException".to_string(),
        export_name: Some("default".to_string()),
        type_only: false,
      }],
    }))
    .add_shim(Shim::Package(PackageShim {
      package: PackageMappedSpecifier {
        name: "buffer".to_string(),
        version: None,
        sub_path: None,
        peer_dependency: false,
      },
      types_package: None,
      global_names: vec![
        GlobalName {
          name: "Blob".to_string(),
          export_name: None,
          type_only: false,
        },
        GlobalName {
          name: "Other".to_string(),
          export_name: None,
          type_only: true,
        },
      ],
    }))
    .add_shim(Shim::Package(PackageShim {
      package: PackageMappedSpecifier {
        name: "type-only".to_string(),
        version: None,
        sub_path: None,
        peer_dependency: false,
      },
      types_package: None,
      global_names: vec![GlobalName {
        name: "TypeOnly".to_string(),
        export_name: None,
        type_only: true,
      }],
    }))
    .add_shim(Shim::Module(ModuleShim {
      module: "bare-module".to_string(),
      global_names: vec![GlobalName {
        name: "BareModule".to_string(),
        export_name: None,
        type_only: false,
      }],
    }))
    .add_shim(Shim::Module(ModuleShim {
      module: "file:///local_shim.ts".to_string(),
      global_names: vec![GlobalName {
        name: "LocalShim".to_string(),
        export_name: None,
        type_only: false,
      }],
    }))
    .add_shim(Shim::Module(ModuleShim {
      module: "https:///localhost/remote_shim.ts".to_string(),
      global_names: vec![GlobalName {
        name: "RemoteShim".to_string(),
        export_name: None,
        type_only: false,
      }],
    }))
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      (
        "_dnt.shims.ts",
        get_shim_file_text(
          concat!(
            "import { default as fetch } from \"node-fetch\";\n",
            "export { default as fetch } from \"node-fetch\";\n",
            "import { fetchTestName as fetchTest } from \"node-fetch/test\";\n",
            "export { fetchTestName as fetchTest } from \"node-fetch/test\";\n",
            "import { default as DOMException } from \"domexception\";\n",
            "export { default as DOMException } from \"domexception\";\n",
            "import { Blob } from \"buffer\";\n",
            "export { Blob, type Other } from \"buffer\";\n",
            "export { type TypeOnly } from \"type-only\";\n",
            "import { BareModule } from \"bare-module\";\n",
            "export { BareModule } from \"bare-module\";\n",
            "import { LocalShim } from \"./local_shim.js\";\n",
            "export { LocalShim } from \"./local_shim.js\";\n",
            "import { RemoteShim } from \"./deps/localhost/remote_shim.js\";\n",
            "export { RemoteShim } from \"./deps/localhost/remote_shim.js\";\n",
            "\n",
            "const dntGlobals = {\n",
            "  fetch,\n",
            "  fetchTest,\n",
            "  DOMException,\n",
            "  Blob,\n",
            "  BareModule,\n",
            "  LocalShim,\n",
            "  RemoteShim,\n",
            "};\n",
            "export const dntGlobalThis = createMergeProxy(globalThis, dntGlobals);\n",
          ).to_string(),
        ),
      ),
      (
        "mod.ts",
        concat!(
          "import * as dntShim from \"./_dnt.shims.js\";\n",
          "dntShim.fetch(); console.log(dntShim.Blob); dntShim.fetchTest(); dntShim.DOMException; dntShim.BareModule; dntShim.LocalShim;"
        )
        .to_string()
      ),
      (
        "local_shim.ts",
        concat!(
          "import * as dntShim from \"./_dnt.shims.js\";\n",
          "export class LocalShim {} dntShim.fetch()"
        ).to_string(),
      ),
      (
        "deps/localhost/remote_shim.ts",
        concat!(
          "import * as dntShim from \"../../_dnt.shims.js\";\n",
          "export class RemoteShim {} dntShim.fetch()"
        ).to_string(),
      ),
    ]
  );
  assert_eq!(
    result.main.dependencies,
    vec![
      Dependency {
        name: "node-fetch".to_string(),
        version: "~3.1.0".to_string(),
        peer_dependency: false,
      },
      Dependency {
        name: "domexception".to_string(),
        version: "^4.0.0".to_string(),
        peer_dependency: false,
      }
    ]
  );
  assert_eq!(
    result.test.dependencies,
    vec![Dependency {
      name: "@types/domexception".to_string(),
      version: "^2.0.1".to_string(),
      peer_dependency: false,
    }]
  );
}

#[tokio::test]
async fn transform_shim_node_custom_shims() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file(
        "/mod.ts",
        "const readableStream = new ReadableStream();",
      );
    })
    .add_shim(Shim::Module(ModuleShim {
      module: "node:stream/web".to_string(),
      global_names: vec![GlobalName {
        name: "ReadableStream".to_string(),
        export_name: None,
        type_only: false,
      }],
    }))
    .transform()
    .await
    .unwrap();

  assert_files!(result.main.files, &[
    (
      "_dnt.shims.ts",
      get_shim_file_text(
        concat!(
          "import { ReadableStream } from \"node:stream/web\";\n",
          "export { ReadableStream } from \"node:stream/web\";\n",
          "\n",
          "const dntGlobals = {\n",
          "  ReadableStream,\n",
          "};\n",
          "export const dntGlobalThis = createMergeProxy(globalThis, dntGlobals);\n",
        ).to_string(),
      ),
    ),
    (
      "mod.ts",
      concat!(
        "import * as dntShim from \"./_dnt.shims.js\";\n",
        "const readableStream = new dntShim.ReadableStream();"
      ).to_string()
    )
  ]);
}

#[tokio::test]
async fn no_transform_deno_ignored() {
  assert_identity_transforms(vec!["// dnt-shim-ignore\nDeno.readTextFile();"])
    .await;
}

#[tokio::test]
async fn transform_legacy_deno_shim_ignore_warnings() {
  // this was renamed to dnt-shim-ignore
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "// deno-shim-ignore\nDeno.readTextFile();");
    })
    .transform()
    .await
    .unwrap();

  assert_eq!(result.warnings, vec!["deno-shim-ignore has been renamed to dnt-shim-ignore. Please rename it in file:///mod.ts"]);
  assert_files!(
    result.main.files,
    &[("mod.ts", "// deno-shim-ignore\nDeno.readTextFile();")]
  );
}

#[tokio::test]
async fn transform_global_this_shim() {
  assert_transforms(vec![(
    concat!(
      "globalThis.Deno.readTextFile();",
      "globalThis.test();",
      "globalThis.test.test();",
      "globalThis['test']();",
      r#"globalThis["test"]();"#,
      "globalThis.Deno = 5;",
      "true ? globalThis : globalThis;",
      "typeof globalThis.Deno;",
      "'Deno' in globalThis;",
      "typeof globalThis;",
      "globalThis == null;",
      "globalThis ? true : false;",
      "type Test1 = typeof globalThis;",
      "type Test2 = typeof globalThis.Window;",
      "type Test3 = typeof globalThis.Deno;",
      "type Test4 = window.Something;",
    ),
    concat!(
      r#"import * as dntShim from "./_dnt.shims.js";"#,
      "\ndntShim.dntGlobalThis.Deno.readTextFile();",
      "globalThis.test();",
      "globalThis.test.test();",
      "globalThis['test']();",
      r#"globalThis["test"]();"#,
      "dntShim.dntGlobalThis.Deno = 5;",
      "true ? dntShim.dntGlobalThis : dntShim.dntGlobalThis;",
      "typeof dntShim.dntGlobalThis.Deno;",
      "'Deno' in dntShim.dntGlobalThis;",
      "typeof dntShim.dntGlobalThis;",
      "dntShim.dntGlobalThis == null;",
      "dntShim.dntGlobalThis ? true : false;",
      "type Test1 = typeof dntShim.dntGlobalThis;",
      "type Test2 = typeof globalThis.Window;",
      "type Test3 = typeof dntShim.Deno;",
      "type Test4 = globalThis.Something;",
    ),
  )])
  .await;
}

#[tokio::test]
async fn transform_window() {
  assert_transforms(vec![
    (
      concat!("window.test = 5;", "window.Deno.test();",),
      concat!(
        r#"import * as dntShim from "./_dnt.shims.js";"#,
        "\nglobalThis.test = 5;",
        "dntShim.dntGlobalThis.Deno.test();",
      ),
    ),
    (
      // should be as-is because there's a declaration
      "const window = {}; window.test;",
      "const window = {}; window.test;",
    ),
  ])
  .await;
}

#[tokio::test]
async fn no_shim_situations() {
  assert_identity_transforms(vec![
    "const { Deno } = test; Deno.test;",
    "const [ Deno ] = test; Deno.test;",
    "const { asdf, ...Deno } = test;",
    "const { Deno: test } = test;",
    "const { test: Deno } = test;",
    "const [Deno] = test;",
    "const [test, ...Deno] = test;",
    "const obj = { Deno: test };",
    "interface Deno {} function test(d: Deno) {}",
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
    "interface Response {} function test(r: Response) {}",
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
      r#"import * as dntShim from "./_dnt.shims.js";"#,
      "\nconst Deno = {};",
      "const { Deno: Deno2 } = dntShim.dntGlobalThis;",
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
            "import 'https://deno.land/std@0.143.0/mod.ts';",
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
          "https://deno.land/std@0.143.0/mod.ts",
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
          "import './deps/deno.land/std@0.143.0/mod.js';",
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
      ("deps/deno.land/std@0.143.0/mod.ts", "console.log(5);"),
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

  assert_eq!(
    err_message.to_string(),
    r#"Module not found "file:///other.ts"."#
  );
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
    r#"Module not found "http://localhost/other.ts"."#
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

  assert_eq!(err_message.to_string(), "The module's source code could not be parsed: Expected ';', '}' or <eof> at http://localhost/declarations.d.ts:1:6");
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
      ("mod.ts", "\nexport * from './deps/localhost/mod.js';"),
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
      ("deps/localhost/mod.js", "\nfunction test() { return 5; }"),
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
        "\nexport * from './file.js';\n\nexport * as test2 from './file.js';\nexport * from './other.js';"
      ),
      (
        "other.ts",
        "\nexport * as other from './file.js';"
      ),
      ("file.js", "\nfunction test() { return 5; }"),
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
        "\nexport * from './file.js';\nexport * from './other.js';"
      ),
      ("other.ts", "\nexport * as other from './file.js';"),
      ("file.js", "\nfunction test() { return 5; }"),
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
        "\nexport * from './file.js';\nexport * from './other.js';"
      ),
      (
        "deps/localhost/other.ts",
        "\nexport * as other from './file.js';"
      ),
      ("deps/localhost/file.js", "\nfunction test() { return 5; }"),
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
            "import * as local from './file.ts';\n",
            "import * as entryA from 'http://localhost/mod/entryA.ts';\n",
            "import * as entryB from 'http://localhost/mod/entryB.ts';\n",
            "import * as entryC from 'http://localhost/mod/entryC.ts';\n",
          ),
        )
        .add_remote_file(
          "http://localhost/mod.ts",
          "import * as myOther from './other.ts';",
        );
    })
    .add_package_specifier_mapping(
      "http://localhost/mod.ts",
      "remote-module",
      Some("1.0.0"),
      None,
    )
    .add_package_specifier_mapping(
      "file:///file.ts",
      "local-module",
      None,
      None,
    )
    .add_package_specifier_mapping(
      "http://localhost/mod/entryA.ts",
      "mod",
      Some("~0.1.0"),
      None,
    )
    .add_package_specifier_mapping(
      "http://localhost/mod/entryB.ts",
      "mod",
      Some("~0.1.0"),
      Some("entryB"),
    )
    .add_package_specifier_mapping(
      "http://localhost/mod/entryC.ts",
      "mod",
      Some("~0.1.0"),
      Some("other/entryC.js"),
    )
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[(
      "mod.ts",
      concat!(
        "import * as remote from 'remote-module';\n",
        "import * as local from 'local-module';\n",
        "import * as entryA from 'mod';\n",
        "import * as entryB from 'mod/entryB';\n",
        "import * as entryC from 'mod/other/entryC.js';\n",
      )
    )]
  );
  assert_eq!(
    result.main.dependencies,
    &[
      Dependency {
        name: "mod".to_string(),
        version: "~0.1.0".to_string(),
        peer_dependency: false,
      },
      Dependency {
        name: "remote-module".to_string(),
        version: "1.0.0".to_string(),
        peer_dependency: false,
      }
    ]
  );
}

#[tokio::test]
async fn transform_not_found_mappings() {
  let error_message = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", "test");
    })
    .add_package_specifier_mapping(
      "http://localhost/mod.ts",
      "local-module",
      None,
      None,
    )
    .add_package_specifier_mapping(
      "http://localhost/mod2.ts",
      "local-module2",
      None,
      None,
    )
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(
    error_message.to_string(),
    "The following specifiers were indicated to be mapped to a package, but were not found:\n  * http://localhost/mod.ts\n  * http://localhost/mod2.ts"
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
            "import * as path from 'https://deno.land/std@0.143.0/node/path.ts';\n",
            "import { performance } from 'https://deno.land/std@0.156.0/node/perf_hooks.ts';\n",
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
        "import { performance } from 'perf_hooks';\n",
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
            "import package6 from 'https://cdn.skypack.dev/preact@^10.5.0/hooks?dts';\n",
            "import package7 from 'https://esm.sh/react-dom@17.0.2/server';\n",
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
          "import package4 from './deps/esm.sh/swr.js';\n",
          "import package5 from './deps/esm.sh/test@1.2.5.js';\n",
          "import package6 from 'preact/hooks';\n",
          "import package7 from 'react-dom/server';\n",
        )
      ),
      ("deps/esm.sh/swr.ts", "",),
      ("deps/esm.sh/test@1.2.5.ts", "",)
    ]
  );
  assert_eq!(
    result.main.dependencies,
    &[
      Dependency {
        name: "@scope/package-name".to_string(),
        version: "1".to_string(),
        peer_dependency: false,
      },
      Dependency {
        name: "preact".to_string(),
        version: "^10.5.0".to_string(),
        peer_dependency: false,
      },
      Dependency {
        name: "react".to_string(),
        version: "17.0.2".to_string(),
        peer_dependency: false,
      },
      Dependency {
        name: "react-dom".to_string(),
        version: "17.0.2".to_string(),
        peer_dependency: false,
      }
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
async fn esm_module_with_deno_types() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          concat!(
            "// @deno-types=\"https://localhost/mod.d.ts\"\n",
            "import {test} from 'https://esm.sh/test@0.0.1/lib/mod.js';\n",
          ),
        )
        .add_remote_file_with_headers(
          "https://esm.sh/test@0.0.1/lib/mod.js",
          "export function test() {return 5;}",
          &[("content-type", "application/typescript")],
        )
        .add_remote_file_with_headers(
          "https://localhost/mod.d.ts",
          "declare function test(): number;",
          &[("content-type", "application/typescript")],
        );
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      // this is a bug... it should create a proxy here instead,
      // but will wait for someone to open this as it's probably
      // rare for this to occur in the wild
      ("mod.ts", "\nimport {test} from 'test/lib/mod.js';\n"),
      (
        "deps/localhost/mod.d.ts",
        "declare function test(): number;",
      )
    ]
  );
}

#[tokio::test]
async fn transform_import_map() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          "import * as remote from 'localhost/mod.ts';",
        )
        .add_local_file(
          "/import_map.json",
          r#"{
  "imports": {
    "localhost/": "/subdir/"
  }
}"#,
        )
        .add_local_file(
          "/subdir/mod.ts",
          "import * as myOther from './other.ts';",
        )
        .add_local_file("/subdir/other.ts", "export function test() {}");
    })
    .set_import_map("file:///import_map.json")
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", "import * as remote from './subdir/mod.js';",),
      ("subdir/mod.ts", "import * as myOther from './other.js';",),
      ("subdir/other.ts", "export function test() {}",)
    ]
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
    .add_default_shims()
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
      peer_dependency: false,
    },]
  );
  assert_eq!(result.main.entry_points, &[PathBuf::from("mod.ts")]);

  assert_files!(
    result.test.files,
    &[
      (
        "mod.test.ts",
        concat!(
          "import * as dntShim from \"./_dnt.test_shims.js\";\n",
          "import './mod.js';\n",
          "import package1 from 'preact';\n",
          "import package3 from 'react';\n",
          "dntShim.Deno.writeTextFile('test', 'test')"
        )
        .to_string(),
      ),
      (
        "_dnt.test_shims.ts",
        get_shim_file_text(
          concat!(
            "import { Deno } from \"@deno/shim-deno\";\n",
            "export { Deno } from \"@deno/shim-deno\";\n",
            "import { setTimeout, setInterval } from \"@deno/shim-timers\";\n",
            "export { setTimeout, setInterval } from \"@deno/shim-timers\";\n",
            "\n",
            "const dntGlobals = {\n",
            "  Deno,\n",
            "  setTimeout,\n",
            "  setInterval,\n",
            "};\n",
            "export const dntGlobalThis = createMergeProxy(globalThis, dntGlobals);\n",
          )
          .to_string(),
        ),
      )
    ]
  );
  assert_eq!(
    result.test.dependencies,
    &[
      Dependency {
        name: "react".to_string(),
        version: "17.0.2".to_string(),
        peer_dependency: false,
      },
      Dependency {
        name: "@deno/shim-deno".to_string(),
        version: "^0.1.0".to_string(),
        peer_dependency: false,
      },
      Dependency {
        name: "@deno/shim-timers".to_string(),
        version: "^0.1.0".to_string(),
        peer_dependency: false,
      }
    ]
  );
  assert_eq!(result.test.entry_points, &[PathBuf::from("mod.test.ts")]);
}

#[tokio::test]
async fn test_entry_points_same_module_multiple_places() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          concat!(
            "export * from 'https://deno.land/std@0.143.0/path.ts';\n",
            "import * as deps from './deps.ts';",
          ),
        )
        // ensure that the path.ts in this file being already analyzed
        // doesn't cause flags.ts to not be analyzed
        .add_local_file(
          "/deps.ts",
          concat!(
            "export * from 'https://deno.land/std@0.143.0/path.ts';\n",
            "export * from 'https://deno.land/std@0.143.0/flags.ts';",
          ),
        )
        .add_remote_file(
          "https://deno.land/std@0.143.0/flags.ts",
          "export class Flags {}",
        )
        .add_remote_file(
          "https://deno.land/std@0.143.0/path.ts",
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
          "export * from './deps/deno.land/std@0.143.0/path.js';\n",
          "import * as deps from './deps.js';",
        )
      ),
      (
        "deps.ts",
        concat!(
          "export * from './deps/deno.land/std@0.143.0/path.js';\n",
          "export * from './deps/deno.land/std@0.143.0/flags.js';",
        )
      ),
      (
        "deps/deno.land/std@0.143.0/flags.ts",
        "export class Flags {}"
      ),
      ("deps/deno.land/std@0.143.0/path.ts", "export class Path {}")
    ]
  );
  assert_eq!(result.main.entry_points, &[PathBuf::from("mod.ts")]);

  assert_files!(
    result.test.files,
    &[("mod.test.ts", "import * as deps from './deps.js';",)]
  );
  assert_eq!(result.test.entry_points, &[PathBuf::from("mod.test.ts")]);
}

#[tokio::test]
async fn polyfills_all() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          concat!(
            "export const test = (obj) => Object.hasOwn(obj, 'test');\n",
            "try {\n",
            "} catch (err) {\n",
            "  err.cause = new Error();\n",
            "}\n",
            "''.replaceAll('test', 'other');\n",
            "[].findLast(() => true);\n",
          ),
        )
        .add_local_file("/mod.test.ts", "import * as mod from './mod.ts';");
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
          "import \"./_dnt.polyfills.js\";\n",
          "export const test = (obj) => Object.hasOwn(obj, 'test');\n",
          "try {\n",
          "} catch (err) {\n",
          "  err.cause = new Error();\n",
          "}\n",
          "''.replaceAll('test', 'other');\n",
          "[].findLast(() => true);\n",
        ),
      ),
      (
        "_dnt.polyfills.ts",
        concat!(
          include_str!("../src/polyfills/scripts/esnext.object-has-own.ts"),
          include_str!("../src/polyfills/scripts/esnext.error-cause.ts"),
          include_str!("../src/polyfills/scripts/es2021.string-replaceAll.ts"),
          include_str!("../src/polyfills/scripts/esnext.array-findLast.ts"),
        )
      ),
    ]
  );
  assert_eq!(result.main.entry_points, &[PathBuf::from("mod.ts")]);

  assert_files!(
    result.test.files,
    &[("mod.test.ts", concat!("import * as mod from './mod.js';",),)]
  );
  assert_eq!(result.test.entry_points, &[PathBuf::from("mod.test.ts")]);
}

#[tokio::test]
async fn polyfills_string_replaceall_target() {
  test_string_replace_all_polyfill(ScriptTarget::ES2020, true).await;
  test_string_replace_all_polyfill(ScriptTarget::ES2021, false).await;
}

async fn test_string_replace_all_polyfill(
  target: ScriptTarget,
  should_have_polyfill: bool,
) {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "''.replaceAll('test', 'other');\n")
        .add_local_file("/mod.test.ts", "import * as mod from './mod.ts';");
    })
    .add_test_entry_point("file:///mod.test.ts")
    .set_target(target)
    .transform()
    .await
    .unwrap();

  if should_have_polyfill {
    assert_files!(
      result.main.files,
      &[
        (
          "mod.ts",
          concat!(
            "import \"./_dnt.polyfills.js\";\n",
            "''.replaceAll('test', 'other');\n",
          ),
        ),
        (
          "_dnt.polyfills.ts",
          concat!(include_str!(
            "../src/polyfills/scripts/es2021.string-replaceAll.ts"
          ),)
        ),
      ]
    );
  } else {
    assert_files!(
      result.main.files,
      &[("mod.ts", "''.replaceAll('test', 'other');\n",)]
    );
  }
  assert_eq!(result.main.entry_points, &[PathBuf::from("mod.ts")]);
}

#[tokio::test]
async fn polyfills_test_files() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", "").add_local_file(
        "/mod.test.ts",
        "// Some copyright text\nObject.hasOwn({}, 'prop');",
      );
    })
    .add_test_entry_point("file:///mod.test.ts")
    .transform()
    .await
    .unwrap();

  assert_files!(result.main.files, &[("mod.ts", "",)]);
  assert_eq!(result.main.entry_points, &[PathBuf::from("mod.ts")]);

  assert_files!(
    result.test.files,
    &[
      (
        "mod.test.ts",
        concat!(
          "// Some copyright text\n",
          "import \"./_dnt.test_polyfills.js\";\n\n",
          "Object.hasOwn({}, 'prop');"
        )
      ),
      (
        "_dnt.test_polyfills.ts",
        include_str!("../src/polyfills/scripts/esnext.object-has-own.ts"),
      )
    ]
  );
  assert_eq!(result.test.entry_points, &[PathBuf::from("mod.test.ts")]);
}

#[tokio::test]
async fn polyfills_object_has_own_conflict() {
  // should not do a polyfill because of Object
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "export class Object {} Object.hasOwn();");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[("mod.ts", "export class Object {} Object.hasOwn();")]
  );
}

#[tokio::test]
async fn module_specifier_mapping_general() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "import './other.deno.ts';")
        .add_local_file("/other.deno.ts", "console.log(5);")
        .add_local_file(
          "/other.node.ts",
          concat!(
            "import * as fs from 'fs';\n",
            "import { myFunction } from './myFunction.ts'\n",
            "export function test() {\n",
            "  // dnt-shim-ignore\n",
            "  Deno.readFileSync('test');\n",
            "  Object.hasOwn({}, 'prop');\n",
            "}",
          ),
        )
        .add_local_file("/myFunction.ts", "export function myFunction() {}");
    })
    .add_module_specifier_mapping(
      "file:///other.deno.ts",
      "file:///other.node.ts",
    )
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      (
        "mod.ts",
        concat!(
          "import \"./_dnt.polyfills.js\";\n",
          "import './other.node.js';"
        ),
      ),
      (
        "other.node.ts",
        concat!(
          "import * as fs from 'fs';\n",
          "import { myFunction } from './myFunction.js'\n",
          "export function test() {\n",
          "  // dnt-shim-ignore\n",
          "  Deno.readFileSync('test');\n",
          "  Object.hasOwn({}, 'prop');\n",
          "}",
        )
      ),
      ("myFunction.ts", "export function myFunction() {}",),
      (
        "_dnt.polyfills.ts",
        include_str!("../src/polyfills/scripts/esnext.object-has-own.ts")
      ),
    ]
  );
  assert_eq!(result.main.entry_points, &[PathBuf::from("mod.ts")]);
}

#[tokio::test]
async fn redirect_entrypoint() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.deno.ts", "console.log(5);")
        .add_local_file("/mod.node.ts", "5;");
    })
    .entry_point("file:///mod.deno.ts")
    .add_module_specifier_mapping("file:///mod.deno.ts", "file:///mod.node.ts")
    .transform()
    .await
    .unwrap();

  assert_files!(result.main.files, &[("mod.node.ts", "5;")]);
  assert_eq!(result.main.entry_points, &[PathBuf::from("mod.node.ts")]);
}

#[tokio::test]
async fn redirect_not_found() {
  let err_message = TestBuilder::new()
    .with_loader(|loader| {
      loader.add_local_file("/mod.ts", "console.log(5);");
    })
    .add_module_specifier_mapping("file:///mod.deno.ts", "file:///mod.node.ts")
    .transform()
    .await
    .err()
    .unwrap();

  assert_eq!(
    err_message.to_string(),
    concat!(
      "The following specifiers were indicated to be mapped to a module, but were not found:\n",
      "  * file:///mod.deno.ts",
    ),
  );
}

#[tokio::test]
async fn json_module_import_default() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          r#"import jsonData from './data.json' assert { type: 'json' };"#,
        )
        .add_local_file("/data.json", "\u{FEFF}{ \"prop\": 5 }");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", r#"import jsonData from './data.js';"#),
      ("data.js", r#"export default { "prop": 5 };"#)
    ]
  );
}

#[tokio::test]
async fn json_module_dynamic_import() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          r#"const jsonData = (await import('./data.json', { assert: { type: 'json' } })).default;"#
        )
        .add_local_file("/data.json", r#"{ "prop": 5 }"#);
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      (
        "mod.ts",
        r#"const jsonData = (await import('./data.js')).default;"#
      ),
      ("data.js", r#"export default { "prop": 5 };"#)
    ]
  );
}

#[tokio::test]
async fn json_module_re_export() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          r#"export { default as Test } from './data.json' assert { type: "json" };"#
        )
        .add_local_file("/data.json", r#"{ "prop": 5 }"#);
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", r#"export { default as Test } from './data.js';"#),
      ("data.js", r#"export default { "prop": 5 };"#)
    ]
  );
}

#[tokio::test]
async fn issue_104() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "import type { other } from './test.ts'; import { test } from './test.ts'; test();")
        .add_local_file("/test.ts", "export function test() {} export type other = string;");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", "import type { other } from './test.js'; import { test } from './test.js'; test();"),
      ("test.ts", "export function test() {} export type other = string;"),
    ]
  );
}

#[tokio::test]
async fn local_declaration_file_import() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file("/mod.ts", "import type { A } from './types.d.ts';")
        .add_local_file("/types.d.ts", "export interface A {}");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", "import type { A } from './types';"),
      ("types.d.ts", "export interface A {}"),
    ]
  );
}

#[tokio::test]
async fn remote_declaration_file_import() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          concat!(
            "import type { RawSourceMap } from 'https://esm.sh/source-map@0.7.3/source-map.d.ts';\n",
            "import type { Other } from 'https://localhost/source-map.d.ts';",
          )
        )
        .add_remote_file("https://esm.sh/source-map@0.7.3/source-map.d.ts", "export interface RawSourceMap {}")
        .add_remote_file("https://localhost/source-map.d.ts", "export interface Other {}");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(result.main.files, &[
    (
      "mod.ts",
      concat!(
        "import type { RawSourceMap } from './deps/esm.sh/source-map@0.7.3/source-map';\n",
        "import type { Other } from './deps/localhost/source-map';",
    )),
    ("deps/esm.sh/source-map@0.7.3/source-map.d.ts", "export interface RawSourceMap {}"),
    ("deps/localhost/source-map.d.ts", "export interface Other {}"),
  ]);
}

#[tokio::test]
async fn import_type_change_specifier() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          r#"export type Test = import('./other.ts').Test"#,
        )
        .add_local_file("/other.ts", "export type Test = string;");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      ("mod.ts", r#"export type Test = import('./other.js').Test"#),
      ("other.ts", "export type Test = string;")
    ]
  );
}

#[tokio::test]
async fn module_decl_string_literal_change_specifier() {
  let result = TestBuilder::new()
    .with_loader(|loader| {
      loader
        .add_local_file(
          "/mod.ts",
          r#"import Test from './other.ts'; declare module './other.ts' {}"#,
        )
        .add_local_file("/other.ts", "export type Test = string;");
    })
    .transform()
    .await
    .unwrap();

  assert_files!(
    result.main.files,
    &[
      (
        "mod.ts",
        r#"import Test from './other.js'; declare module './other.js' {}"#
      ),
      ("other.ts", "export type Test = string;")
    ]
  );
}

fn get_shim_file_text(mut text: String) -> String {
  text.push('\n');
  text.push_str(
    &include_str!("../src/scripts/createMergeProxy.ts")
      .replace("export function", "function"),
  );
  text
}
