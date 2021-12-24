// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "./test.deps.ts";
import { getPackageJson, GetPackageJsonOptions } from "./package_json.ts";

const versions = {
  chalk: "4.1.2",
  nodeTypes: "16.11.1",
  tsLib: "2.3.1",
};

Deno.test("single entrypoint", () => {
  const props: GetPackageJsonOptions = {
    transformOutput: {
      main: {
        files: [],
        dependencies: [{
          name: "dep",
          version: "^1.0.0",
        }],
        entryPoints: ["mod.ts"],
      },
      test: {
        entryPoints: [],
        files: [],
        dependencies: [{
          name: "test-dep",
          version: "0.1.0",
        }, {
          name: "@deno/shim-deno",
          version: "~0.1.0",
        }],
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
    testEnabled: true,
    includeCjs: true,
    includeDeclarations: true,
    includeTsLib: false,
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
      "@types/node": versions.nodeTypes,
      "chalk": versions.chalk,
      "test-dep": "0.1.0",
      "@deno/shim-deno": "~0.1.0",
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
      transformOutput: props.transformOutput,
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

  assertEquals(
    getPackageJson({
      ...props,
      testEnabled: false,
      includeCjs: false,
    }),
    {
      name: "package",
      version: "0.1.0",
      main: undefined,
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
          require: undefined,
          types: "./types/mod.d.ts",
        },
      },
    },
  );

  assertEquals(
    getPackageJson({
      ...props,
      testEnabled: false,
      includeCjs: false,
      includeDeclarations: false,
    }),
    {
      name: "package",
      version: "0.1.0",
      main: undefined,
      module: "./esm/mod.js",
      types: undefined,
      dependencies: {
        dep: "^1.0.0",
      },
      devDependencies: {},
      scripts: undefined,
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: undefined,
          types: undefined,
        },
      },
    },
  );

  // tslib
  assertEquals(
    getPackageJson({
      ...props,
      testEnabled: false,
      includeCjs: false,
      includeDeclarations: false,
      includeTsLib: true,
    }),
    {
      name: "package",
      version: "0.1.0",
      main: undefined,
      module: "./esm/mod.js",
      types: undefined,
      dependencies: {
        tslib: versions.tsLib,
        dep: "^1.0.0",
      },
      devDependencies: {},
      scripts: undefined,
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: undefined,
          types: undefined,
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
        dependencies: [{
          name: "@deno/shim-deno",
          version: "~0.1.0",
        }],
        entryPoints: ["mod.ts", "other.ts"],
      },
      test: {
        entryPoints: [],
        files: [],
        dependencies: [],
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
    testEnabled: false,
    includeCjs: true,
    includeDeclarations: true,
    includeTsLib: false,
  };

  assertEquals(getPackageJson(props), {
    name: "package",
    version: "0.1.0",
    main: "./umd/mod.js",
    module: "./esm/mod.js",
    types: "./types/mod.d.ts",
    dependencies: {
      "@deno/shim-deno": "~0.1.0",
    },
    devDependencies: {
      "@types/node": versions.nodeTypes,
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

Deno.test("binary entrypoints", () => {
  const props: GetPackageJsonOptions = {
    transformOutput: {
      main: {
        files: [],
        dependencies: [{
          name: "@deno/shim-deno",
          version: "~0.1.0",
        }],
        entryPoints: ["mod.ts", "bin.ts"],
      },
      test: {
        entryPoints: [],
        files: [],
        dependencies: [],
      },
      warnings: [],
    },
    entryPoints: [{
      name: ".",
      path: "./mod.ts",
    }, {
      kind: "bin",
      name: "my_bin",
      path: "./bin.ts",
    }],
    package: {
      name: "package",
      version: "0.1.0",
    },
    testEnabled: false,
    includeCjs: true,
    includeDeclarations: true,
    includeTsLib: false,
  };

  assertEquals(getPackageJson(props), {
    name: "package",
    version: "0.1.0",
    main: "./umd/mod.js",
    module: "./esm/mod.js",
    types: "./types/mod.d.ts",
    bin: {
      my_bin: "./esm/bin.js",
    },
    dependencies: {
      "@deno/shim-deno": "~0.1.0",
    },
    devDependencies: {
      "@types/node": versions.nodeTypes,
    },
    exports: {
      ".": {
        import: "./esm/mod.js",
        require: "./umd/mod.js",
        types: "./types/mod.d.ts",
      },
    },
    scripts: undefined,
  });
});
