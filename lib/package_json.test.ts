// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "./test.deps.ts";
import { getPackageJson, GetPackageJsonOptions } from "./package_json.ts";

Deno.test("single entrypoint", () => {
  const props: GetPackageJsonOptions = {
    transformOutput: {
      main: {
        files: [],
        dependencies: [{
          name: "dep",
          version: "^1.0.0",
        }],
        shimUsed: false,
        entryPoints: ["mod.ts"],
      },
      test: {
        entryPoints: [],
        files: [],
        dependencies: [{
          name: "test-dep",
          version: "0.1.0",
        }],
        shimUsed: true,
      },
      warnings: [],
    },
    entryPoints: [{
      name: ".",
      path: "./mod.ts",
    }],
    package: {
      name: "package",
      version: "0.1.0",
    },
    shimPackage: {
      name: "deno.ns",
      version: "0.0.0",
    },
    testEnabled: true,
  };

  assertEquals(getPackageJson(props), {
    name: "package",
    version: "0.1.0",
    main: "./umd/mod.js",
    module: "./esm/mod.js",
    types: "./types/mod.d.ts",
    dependencies: {
      dep: "^1.0.0",
    },
    devDependencies: {
      "@types/node": "16.11.1",
      "chalk": "4.1.2",
      "deno.ns": "0.0.0",
      "test-dep": "0.1.0",
    },
    exports: {
      ".": {
        import: "./esm/mod.js",
        require: "./umd/mod.js",
        types: "./types/mod.d.ts",
      },
    },
    scripts: {
      test: "node test_runner.js",
    },
  });

  assertEquals(
    getPackageJson({
      ...props,
      transformOutput: {
        ...props.transformOutput,
        test: {
          ...props.transformOutput.test,
          shimUsed: false,
        },
      },
      testEnabled: false,
    }),
    {
      name: "package",
      version: "0.1.0",
      main: "./umd/mod.js",
      module: "./esm/mod.js",
      types: "./types/mod.d.ts",
      dependencies: {
        dep: "^1.0.0",
      },
      devDependencies: {},
      scripts: undefined,
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: "./umd/mod.js",
          types: "./types/mod.d.ts",
        },
      },
    },
  );
});

Deno.test("multiple entrypoints", () => {
  const props: GetPackageJsonOptions = {
    transformOutput: {
      main: {
        files: [],
        dependencies: [],
        shimUsed: true,
        entryPoints: ["mod.ts", "other.ts"],
      },
      test: {
        entryPoints: [],
        files: [],
        dependencies: [],
        shimUsed: false,
      },
      warnings: [],
    },
    entryPoints: [{
      name: ".",
      path: "./mod.ts",
    }, {
      name: "./my-other-entrypoint.js",
      path: "./other.ts",
    }],
    package: {
      name: "package",
      version: "0.1.0",
    },
    shimPackage: {
      name: "deno.ns",
      version: "0.0.0",
    },
    testEnabled: false,
  };

  assertEquals(getPackageJson(props), {
    name: "package",
    version: "0.1.0",
    main: "./umd/mod.js",
    module: "./esm/mod.js",
    types: "./types/mod.d.ts",
    dependencies: {
      "deno.ns": "0.0.0",
    },
    devDependencies: {
      "@types/node": "16.11.1",
    },
    exports: {
      ".": {
        import: "./esm/mod.js",
        require: "./umd/mod.js",
        types: "./types/mod.d.ts",
      },
      "./my-other-entrypoint.js": {
        import: "./esm/other.js",
        require: "./umd/other.js",
        types: "./types/other.d.ts",
      },
    },
    scripts: undefined,
  });
});
