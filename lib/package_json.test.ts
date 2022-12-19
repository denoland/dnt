// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "./test.deps.ts";
import { getPackageJson, GetPackageJsonOptions } from "./package_json.ts";

const versions = {
  chalk: "^4.1.2",
  nodeTypes: "^18.11.9",
  tsLib: "^2.4.1",
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
    includeEsModule: true,
    includeScriptModule: true,
    includeDeclarations: true,
    includeTsLib: false,
    shims: {
      deno: "dev",
    },
  };

  assertEquals(getPackageJson(props), {
    name: "package",
    version: "0.1.0",
    main: "./script/mod.js",
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
        import: {
          default: "./esm/mod.js",
          types: "./types/mod.d.ts",
        },
        require: {
          default: "./script/mod.js",
          types: "./types/mod.d.ts",
        },
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
      main: "./script/mod.js",
      module: "./esm/mod.js",
      types: "./types/mod.d.ts",
      dependencies: {
        dep: "^1.0.0",
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
      },
      scripts: undefined,
      exports: {
        ".": {
          import: {
            default: "./esm/mod.js",
            types: "./types/mod.d.ts",
          },
          require: {
            default: "./script/mod.js",
            types: "./types/mod.d.ts",
          },
        },
      },
    },
  );

  assertEquals(
    getPackageJson({
      ...props,
      testEnabled: false,
      includeScriptModule: false,
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
      devDependencies: {
        "@types/node": versions.nodeTypes,
      },
      scripts: undefined,
      exports: {
        ".": {
          import: {
            default: "./esm/mod.js",
            types: "./types/mod.d.ts",
          },
          require: undefined,
        },
      },
    },
  );

  assertEquals(
    getPackageJson({
      ...props,
      testEnabled: false,
      includeEsModule: false,
    }),
    {
      name: "package",
      version: "0.1.0",
      main: "./script/mod.js",
      module: undefined,
      types: "./types/mod.d.ts",
      dependencies: {
        dep: "^1.0.0",
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
      },
      scripts: undefined,
    },
  );

  assertEquals(
    getPackageJson({
      ...props,
      testEnabled: false,
      includeScriptModule: false,
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
      devDependencies: {
        "@types/node": versions.nodeTypes,
      },
      scripts: undefined,
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: undefined,
        },
      },
    },
  );

  // tslib
  assertEquals(
    getPackageJson({
      ...props,
      testEnabled: false,
      includeScriptModule: false,
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
      devDependencies: {
        "@types/node": versions.nodeTypes,
      },
      scripts: undefined,
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: undefined,
        },
      },
    },
  );
});

Deno.test("exports have default last", () => {
  const props: GetPackageJsonOptions = {
    transformOutput: {
      main: {
        files: [],
        dependencies: [],
        entryPoints: ["mod.ts"],
      },
      test: {
        entryPoints: [],
        files: [],
        dependencies: [],
      },
      warnings: [],
    },
    entryPoints: [
      {
        name: ".",
        path: "./mod.ts",
      },
    ],
    package: {
      name: "package",
      version: "0.1.0",
    },
    testEnabled: true,
    includeEsModule: true,
    includeScriptModule: true,
    includeDeclarations: true,
    includeTsLib: false,
    shims: {
      deno: "dev",
    },
  };

  const result: any = getPackageJson(props);
  assertEquals(Object.keys(result.exports), ["."]);
  assertEquals(Object.keys(result.exports["."]), ["import", "require"]);

  // "default" must always be last: https://github.com/denoland/dnt/issues/228
  assertEquals(Object.keys(result.exports["."].import), ["types", "default"]);
  assertEquals(Object.keys(result.exports["."].require), ["types", "default"]);
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
    includeEsModule: true,
    includeScriptModule: true,
    includeDeclarations: true,
    includeTsLib: false,
    shims: { deno: true },
  };

  assertEquals(getPackageJson(props), {
    name: "package",
    version: "0.1.0",
    main: "./script/mod.js",
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
        import: {
          default: "./esm/mod.js",
          types: "./types/mod.d.ts",
        },
        require: {
          default: "./script/mod.js",
          types: "./types/mod.d.ts",
        },
      },
      "./my-other-entrypoint.js": {
        import: {
          default: "./esm/other.js",
          types: "./types/other.d.ts",
        },
        require: {
          default: "./script/other.js",
          types: "./types/other.d.ts",
        },
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
    includeEsModule: true,
    includeScriptModule: true,
    includeDeclarations: true,
    includeTsLib: false,
    shims: { deno: true },
  };

  assertEquals(getPackageJson(props), {
    name: "package",
    version: "0.1.0",
    main: "./script/mod.js",
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
        import: {
          default: "./esm/mod.js",
          types: "./types/mod.d.ts",
        },
        require: {
          default: "./script/mod.js",
          types: "./types/mod.d.ts",
        },
      },
    },
    scripts: undefined,
  });
});

Deno.test("peer dependencies", () => {
  const props: GetPackageJsonOptions = {
    transformOutput: {
      main: {
        files: [],
        dependencies: [{
          name: "dep",
          version: "^1.0.0",
        }, {
          name: "peerDep",
          version: "^2.0.0",
          peerDependency: true,
        }],
        entryPoints: ["mod.ts"],
      },
      test: {
        entryPoints: [],
        files: [],
        dependencies: [{
          name: "test-dep",
          version: "0.1.0",
          // should be ignored
          peerDependency: true,
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
    includeEsModule: true,
    includeScriptModule: true,
    includeDeclarations: true,
    includeTsLib: false,
    shims: {
      deno: "dev",
    },
  };

  assertEquals(getPackageJson(props), {
    name: "package",
    version: "0.1.0",
    main: "./script/mod.js",
    module: "./esm/mod.js",
    types: "./types/mod.d.ts",
    dependencies: {
      dep: "^1.0.0",
    },
    peerDependencies: {
      peerDep: "^2.0.0",
    },
    devDependencies: {
      "@types/node": versions.nodeTypes,
      "chalk": versions.chalk,
      "test-dep": "0.1.0",
      "@deno/shim-deno": "~0.1.0",
    },
    exports: {
      ".": {
        import: {
          default: "./esm/mod.js",
          types: "./types/mod.d.ts",
        },
        require: {
          default: "./script/mod.js",
          types: "./types/mod.d.ts",
        },
      },
    },
    scripts: {
      test: "node test_runner.js",
    },
  });
});
