// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals, assertRejects, assertStringIncludes } from "@std/assert";
import * as path from "@std/path";
import type { ShimValue } from "../lib/shims.ts";
import { build, type BuildOptions, type ShimOptions } from "../mod.ts";

const versions = {
  denoShim: "~0.18.0",
  denoTestShim: "~0.5.0",
  cryptoShim: "~0.3.1",
  domExceptionShim: "^4.0.0",
  domExceptionShimTypes: "^4.0.0",
  promptsShim: "~0.1.0",
  weakRefSham: "~0.1.0",
  undici: "^6.0.0",
  picocolors: "^1.0.0",
  nodeTypes: "^20.9.0",
  newNodeTypes: "^22.16.3",
  tsLib: "^2.6.2",
};

Deno.test("should throw because both scriptModule and esModule are false", async () => {
  await assertRejects(() =>
    runTest("test_project", {
      entryPoints: ["mod.ts"],
      outDir: "./npm",
      scriptModule: false,
      esModule: false,
      shims: {
        ...getAllShimOptions(false),
        deno: "dev",
        weakRef: true,
      },
      package: {
        name: "add",
        version: "1.0.0",
      },
    })
  );
});

Deno.test("should build test project - basic", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    typeCheck: "both",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
      weakRef: true,
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
    compilerOptions: {
      importHelpers: true,
    },
  }, (output) => {
    output.assertNotExists("script/mod.js.map");
    output.assertNotExists("esm/mod.js.map");
    assertEquals(output.packageJson, {
      name: "add",
      version: "1.0.0",
      main: "./script/mod.js",
      module: "./esm/mod.js",
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: "./script/mod.js",
        },
      },
      scripts: {
        test: "node test_runner.cjs",
      },
      dependencies: {
        tslib: versions.tsLib,
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
        picocolors: versions.picocolors,
        "@deno/shim-deno": versions.denoShim,
        "@deno/sham-weakref": versions.weakRefSham,
      },
      _generatedBy: "dnt@dev",
    });
    assertEquals(
      output.npmIgnore,
      `/src/
/esm/mod.test.js
/esm/mod.test.d.ts
/script/mod.test.js
/script/mod.test.d.ts
/esm/deps/deno.land/std@0.181.0/fmt/colors.js
/esm/deps/deno.land/std@0.181.0/fmt/colors.d.ts
/script/deps/deno.land/std@0.181.0/fmt/colors.js
/script/deps/deno.land/std@0.181.0/fmt/colors.d.ts
/esm/deps/deno.land/std@0.181.0/testing/_diff.js
/esm/deps/deno.land/std@0.181.0/testing/_diff.d.ts
/script/deps/deno.land/std@0.181.0/testing/_diff.js
/script/deps/deno.land/std@0.181.0/testing/_diff.d.ts
/esm/deps/deno.land/std@0.181.0/testing/_format.js
/esm/deps/deno.land/std@0.181.0/testing/_format.d.ts
/script/deps/deno.land/std@0.181.0/testing/_format.js
/script/deps/deno.land/std@0.181.0/testing/_format.d.ts
/esm/deps/deno.land/std@0.181.0/testing/asserts.js
/esm/deps/deno.land/std@0.181.0/testing/asserts.d.ts
/script/deps/deno.land/std@0.181.0/testing/asserts.js
/script/deps/deno.land/std@0.181.0/testing/asserts.d.ts
/esm/_dnt.test_polyfills.js
/esm/_dnt.test_polyfills.d.ts
/script/_dnt.test_polyfills.js
/script/_dnt.test_polyfills.d.ts
/esm/_dnt.test_shims.js
/esm/_dnt.test_shims.d.ts
/script/_dnt.test_shims.js
/script/_dnt.test_shims.d.ts
/test_runner.cjs
yarn.lock
pnpm-lock.yaml
`,
    );
  });
});

Deno.test("should build test project without esm", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    esModule: false,
    declaration: "separate",
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
      weakRef: true,
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
    compilerOptions: {
      importHelpers: true,
    },
  }, (output) => {
    output.assertNotExists("script/mod.js.map");
    output.assertNotExists("esm/mod.js.map");
    assertEquals(output.packageJson, {
      name: "add",
      version: "1.0.0",
      main: "./script/mod.js",
      scripts: {
        test: "node test_runner.cjs",
      },
      types: "./types/mod.d.ts",
      dependencies: {
        tslib: versions.tsLib,
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
        picocolors: versions.picocolors,
        "@deno/shim-deno": versions.denoShim,
        "@deno/sham-weakref": versions.weakRefSham,
      },
      _generatedBy: "dnt@dev",
    });
    assertEquals(
      output.npmIgnore,
      `/src/
/script/mod.test.js
/types/mod.test.d.ts
/script/deps/deno.land/std@0.181.0/fmt/colors.js
/types/deps/deno.land/std@0.181.0/fmt/colors.d.ts
/script/deps/deno.land/std@0.181.0/testing/_diff.js
/types/deps/deno.land/std@0.181.0/testing/_diff.d.ts
/script/deps/deno.land/std@0.181.0/testing/_format.js
/types/deps/deno.land/std@0.181.0/testing/_format.d.ts
/script/deps/deno.land/std@0.181.0/testing/asserts.js
/types/deps/deno.land/std@0.181.0/testing/asserts.d.ts
/script/_dnt.test_polyfills.js
/types/_dnt.test_polyfills.d.ts
/script/_dnt.test_shims.js
/types/_dnt.test_shims.d.ts
/test_runner.cjs
yarn.lock
pnpm-lock.yaml
`,
    );
  });
});

Deno.test("should build with all options off", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: {
        test: true,
      },
    },
    typeCheck: false,
    scriptModule: false,
    declaration: false,
    test: false,
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    assertEquals(output.packageJson, {
      name: "add",
      version: "1.0.0",
      module: "./esm/mod.js",
      exports: {
        ".": {
          import: "./esm/mod.js",
        },
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
      },
      scripts: {},
      _generatedBy: "dnt@dev",
    });

    output.assertNotExists("script/mod.js");
    output.assertNotExists("types/mod.js");

    // This doesn't include the test files because they're not analyzed for in this scenario.
    assertEquals(
      output.npmIgnore,
      `/src/
/test_runner.cjs
yarn.lock
pnpm-lock.yaml
`,
    );
  });
});

Deno.test("should build umd module", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      deno: "dev",
    },
    scriptModule: "umd",
    typeCheck: "both",
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    const fileText = output.getFileText("script/mod.js");
    assertStringIncludes(fileText, "(function (factory) {");
  });
});

Deno.test("should build test project with declarations inline by default", async () => {
  const options = ["inline", undefined] as const;
  for (const declaration of options) {
    await runTest("test_project", {
      entryPoints: ["mod.ts"],
      outDir: "./npm",
      declaration,
      declarationMap: false,
      shims: {
        deno: "dev",
      },
      package: {
        name: "add",
        version: "1.0.0",
      },
      compilerOptions: {
        importHelpers: true,
      },
    }, (output) => {
      output.assertNotExists("script/mod.js.map");
      output.assertNotExists("esm/mod.js.map");
      output.assertNotExists("types/mod.d.ts");
      output.assertNotExists("types/mod.d.ts.map");
      output.assertExists("script/mod.d.ts");
      output.assertNotExists("script/mod.d.ts.map");
      output.assertExists("esm/mod.d.ts");
      output.assertNotExists("esm/mod.d.ts.map");
      assertEquals(output.packageJson, {
        name: "add",
        version: "1.0.0",
        main: "./script/mod.js",
        module: "./esm/mod.js",
        exports: {
          ".": {
            import: "./esm/mod.js",
            require: "./script/mod.js",
          },
        },
        scripts: {
          test: "node test_runner.cjs",
        },
        dependencies: {
          tslib: versions.tsLib,
        },
        devDependencies: {
          "@types/node": versions.nodeTypes,
          picocolors: versions.picocolors,
          "@deno/shim-deno": versions.denoShim,
        },
        _generatedBy: "dnt@dev",
      });
    });
  }
});

Deno.test("should build test project without declaration maps by default", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    declaration: "inline",
    shims: {
      deno: "dev",
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertNotExists("script/mod.d.ts.map");
    output.assertNotExists("esm/mod.d.ts.map");
    // nothing points at the sources, so they're not published
    assertStringIncludes(output.npmIgnore, "/src/\n");
  });
});

Deno.test("should build test project with declaration maps when enabled", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    declaration: "inline",
    declarationMap: true,
    shims: {
      deno: "dev",
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertNotExists("types/mod.d.ts");
    output.assertExists("script/mod.d.ts.map");
    output.assertExists("esm/mod.d.ts.map");
    // the declaration maps point at the sources, so they must be published
    output.assertExists("src/mod.ts");
    assertEquals(output.npmIgnore.includes("/src/\n"), false);
  });

  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    declaration: "separate",
    declarationMap: true,
    shims: {
      deno: "dev",
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertExists("types/mod.d.ts.map");
    output.assertNotExists("script/mod.d.ts.map");
    output.assertNotExists("esm/mod.d.ts.map");
    output.assertExists("src/mod.ts");
    assertEquals(output.npmIgnore.includes("/src/\n"), false);
  });
});

Deno.test("should not create declaration maps when the sources are skipped", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    declaration: "inline",
    declarationMap: true,
    skipSourceOutput: true,
    shims: {
      deno: "dev",
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertNotExists("src/mod.ts");
    output.assertNotExists("script/mod.d.ts.map");
    output.assertNotExists("esm/mod.d.ts.map");
  });
});

Deno.test("should build bin project", async () => {
  await runTest("test_project", {
    entryPoints: [{
      kind: "bin",
      name: "add",
      path: "./mod.ts",
    }],
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    outDir: "./npm",
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    assertEquals(output.packageJson, {
      name: "add",
      version: "1.0.0",
      bin: {
        add: "./esm/mod.js",
      },
      scripts: {
        test: "node test_runner.cjs",
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
        picocolors: versions.picocolors,
        "@deno/shim-deno": versions.denoShim,
      },
      _generatedBy: "dnt@dev",
    });
    const expectedText = "#!/usr/bin/env node\n";
    assertEquals(
      output.getFileText("script/mod.js").substring(0, expectedText.length),
      expectedText,
    );
    assertEquals(
      output.getFileText("esm/mod.js").substring(0, expectedText.length),
      expectedText,
    );
  });
});

Deno.test("should build bin project with a shebang", async () => {
  await runTest("bin_shebang_project", {
    entryPoints: [{
      kind: "bin",
      name: "hello",
      path: "./main.ts",
    }],
    shims: getAllShimOptions(false),
    outDir: "./npm",
    package: {
      name: "hello",
      version: "1.0.0",
    },
    compilerOptions: {
      lib: ["ESNext", "DOM"],
    },
  }, (output) => {
    assertEquals(output.packageJson, {
      name: "hello",
      version: "1.0.0",
      bin: {
        hello: "./esm/main.js",
      },
      scripts: {
        test: "node test_runner.cjs",
      },
      devDependencies: {
        picocolors: versions.picocolors,
      },
      _generatedBy: "dnt@dev",
    });
    const expectedText =
      '#!/usr/bin/env node\n"use strict";\nconsole.log("Hello!");\n';
    assertEquals(
      output.getFileText("script/main.js"),
      expectedText,
    );
    assertEquals(
      output.getFileText("esm/main.js"),
      expectedText,
    );
  });
});

Deno.test("should run tests when using @deno/shim-deno-test shim", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: {
        test: "dev",
      },
      weakRef: true,
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
    compilerOptions: {
      target: "ES2019", // node 12
      importHelpers: true,
    },
  }, (output) => {
    output.assertNotExists("script/mod.js.map");
    output.assertNotExists("esm/mod.js.map");
    assertEquals(output.packageJson.devDependencies, {
      "@types/node": versions.nodeTypes,
      picocolors: versions.picocolors,
      "@deno/shim-deno-test": versions.denoTestShim,
      "@deno/sham-weakref": versions.weakRefSham,
    });
  });
});

Deno.test("error for TLA when emitting CommonJS", async () => {
  await assertRejects(() =>
    runTest("tla_project", {
      entryPoints: ["mod.ts"],
      declaration: "separate",
      shims: {
        ...getAllShimOptions(false),
        deno: "dev",
      },
      outDir: "./npm",
      package: {
        name: "add",
        version: "1.0.0",
      },
    })
  );
});

Deno.test("not error for TLA when not using CommonJS", async () => {
  await runTest("tla_project", {
    entryPoints: ["mod.ts"],
    declaration: "separate",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    outDir: "./npm",
    scriptModule: false, // ok, because cjs is disabled now
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    assertEquals(output.packageJson, {
      name: "add",
      version: "1.0.0",
      module: "./esm/mod.js",
      exports: {
        ".": {
          import: {
            types: "./types/mod.d.ts",
            default: "./esm/mod.js",
          },
        },
      },
      scripts: {
        test: "node test_runner.cjs",
      },
      types: "./types/mod.d.ts",
      devDependencies: {
        "@types/node": versions.nodeTypes,
        picocolors: versions.picocolors,
        "@deno/shim-deno": versions.denoShim,
      },
      _generatedBy: "dnt@dev",
    });
  });
});

Deno.test("should build with source maps", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
    compilerOptions: {
      sourceMap: true,
    },
  }, (output) => {
    output.assertExists("script/mod.js.map");
    output.assertExists("esm/mod.js.map");
    assertEquals(
      output.npmIgnore,
      `/src/mod.test.ts
/esm/mod.test.js
/esm/mod.test.js.map
/esm/mod.test.d.ts
/script/mod.test.js
/script/mod.test.js.map
/script/mod.test.d.ts
/src/deps/deno.land/std@0.181.0/fmt/colors.ts
/esm/deps/deno.land/std@0.181.0/fmt/colors.js
/esm/deps/deno.land/std@0.181.0/fmt/colors.js.map
/esm/deps/deno.land/std@0.181.0/fmt/colors.d.ts
/script/deps/deno.land/std@0.181.0/fmt/colors.js
/script/deps/deno.land/std@0.181.0/fmt/colors.js.map
/script/deps/deno.land/std@0.181.0/fmt/colors.d.ts
/src/deps/deno.land/std@0.181.0/testing/_diff.ts
/esm/deps/deno.land/std@0.181.0/testing/_diff.js
/esm/deps/deno.land/std@0.181.0/testing/_diff.js.map
/esm/deps/deno.land/std@0.181.0/testing/_diff.d.ts
/script/deps/deno.land/std@0.181.0/testing/_diff.js
/script/deps/deno.land/std@0.181.0/testing/_diff.js.map
/script/deps/deno.land/std@0.181.0/testing/_diff.d.ts
/src/deps/deno.land/std@0.181.0/testing/_format.ts
/esm/deps/deno.land/std@0.181.0/testing/_format.js
/esm/deps/deno.land/std@0.181.0/testing/_format.js.map
/esm/deps/deno.land/std@0.181.0/testing/_format.d.ts
/script/deps/deno.land/std@0.181.0/testing/_format.js
/script/deps/deno.land/std@0.181.0/testing/_format.js.map
/script/deps/deno.land/std@0.181.0/testing/_format.d.ts
/src/deps/deno.land/std@0.181.0/testing/asserts.ts
/esm/deps/deno.land/std@0.181.0/testing/asserts.js
/esm/deps/deno.land/std@0.181.0/testing/asserts.js.map
/esm/deps/deno.land/std@0.181.0/testing/asserts.d.ts
/script/deps/deno.land/std@0.181.0/testing/asserts.js
/script/deps/deno.land/std@0.181.0/testing/asserts.js.map
/script/deps/deno.land/std@0.181.0/testing/asserts.d.ts
/src/_dnt.test_polyfills.ts
/esm/_dnt.test_polyfills.js
/esm/_dnt.test_polyfills.js.map
/esm/_dnt.test_polyfills.d.ts
/script/_dnt.test_polyfills.js
/script/_dnt.test_polyfills.js.map
/script/_dnt.test_polyfills.d.ts
/src/_dnt.test_shims.ts
/esm/_dnt.test_shims.js
/esm/_dnt.test_shims.js.map
/esm/_dnt.test_shims.d.ts
/script/_dnt.test_shims.js
/script/_dnt.test_shims.js.map
/script/_dnt.test_shims.d.ts
/test_runner.cjs
yarn.lock
pnpm-lock.yaml
`,
    );
  });
});

Deno.test("should build with package mappings", async () => {
  await runTest("package_mappings_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    declaration: "separate",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "mappings",
      version: "1.2.3",
    },
    mappings: {
      "https://deno.land/x/code_block_writer@11.0.0/mod.ts": {
        name: "code-block-writer",
        version: "=11.0.0",
      },
    },
  }, (output) => {
    assertEquals(output.packageJson, {
      name: "mappings",
      version: "1.2.3",
      main: "./script/mod.js",
      module: "./esm/mod.js",
      exports: {
        ".": {
          import: {
            types: "./types/mod.d.ts",
            default: "./esm/mod.js",
          },
          require: {
            types: "./types/mod.d.ts",
            default: "./script/mod.js",
          },
        },
      },
      scripts: {
        test: "node test_runner.cjs",
      },
      types: "./types/mod.d.ts",
      dependencies: {
        "using-statement": "^0.4",
        "code-block-writer": "=11.0.0",
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
        picocolors: versions.picocolors,
        "@deno/shim-deno": versions.denoShim,
      },
      _generatedBy: "dnt@dev",
    });
    assertEquals(
      output.npmIgnore,
      `/src/
/esm/mod.test.js
/script/mod.test.js
/types/mod.test.d.ts
/esm/_dnt.test_shims.js
/script/_dnt.test_shims.js
/types/_dnt.test_shims.d.ts
/test_runner.cjs
yarn.lock
pnpm-lock.yaml
`,
    );
  });
});

Deno.test("should build with peer dependencies in mappings", async () => {
  await runTest("package_mappings_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    declaration: "separate",
    shims: {
      deno: "dev",
    },
    package: {
      name: "mappings",
      version: "1.2.3",
    },
    mappings: {
      "https://deno.land/x/code_block_writer@11.0.0/mod.ts": {
        name: "code-block-writer",
        version: "=11.0.0",
        peerDependency: true,
      },
    },
  }, (output) => {
    assertEquals(output.packageJson, {
      name: "mappings",
      version: "1.2.3",
      main: "./script/mod.js",
      module: "./esm/mod.js",
      exports: {
        ".": {
          import: {
            types: "./types/mod.d.ts",
            default: "./esm/mod.js",
          },
          require: {
            types: "./types/mod.d.ts",
            default: "./script/mod.js",
          },
        },
      },
      scripts: {
        test: "node test_runner.cjs",
      },
      types: "./types/mod.d.ts",
      peerDependencies: {
        "code-block-writer": "=11.0.0",
      },
      dependencies: {
        "using-statement": "^0.4",
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
        picocolors: versions.picocolors,
        "@deno/shim-deno": versions.denoShim,
      },
      _generatedBy: "dnt@dev",
    });
  });
});

Deno.test("should build shim project with everything enabled", async () => {
  await runTest("shim_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(true),
      custom: [{
        module: "./ArrayBuffer.ts",
        globalNames: ["ArrayBuffer"],
      }],
    },
    package: {
      name: "shim-package",
      version: "1.0.0",
    },
  }, (output) => {
    assertEquals(output.packageJson.dependencies, {
      "@deno/shim-crypto": versions.cryptoShim,
      "@deno/shim-deno": versions.denoShim,
      "@deno/shim-prompts": versions.promptsShim,
      "domexception": versions.domExceptionShim,
      "undici": versions.undici,
    });
    assertEquals(output.packageJson.devDependencies, {
      "@types/domexception": versions.domExceptionShimTypes,
      "@types/node": versions.nodeTypes,
      "picocolors": versions.picocolors,
    });
  });
});

Deno.test("should build shim project when using node-fetch", async () => {
  // try a custom shim
  await runTest("shim_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    scriptModule: false,
    shims: {
      ...getAllShimOptions(true),
      undici: false,
      custom: [{
        package: {
          name: "undici",
          version: versions.undici,
        },
        globalNames: [
          // without fetch
          "File",
          "FormData",
          "Headers",
          "Request",
          "Response",
        ],
      }, {
        package: {
          name: "node-fetch",
          version: "~3.1.0",
        },
        globalNames: [{
          name: "fetch",
          exportName: "default",
        }, {
          name: "RequestInit",
          typeOnly: true,
        }],
      }, {
        module: "ArrayBuffer.ts",
        globalNames: ["ArrayBuffer"],
      }],
    },
    package: {
      name: "shim-package",
      version: "1.0.0",
    },
  }, (output) => {
    assertEquals(output.packageJson.dependencies, {
      "@deno/shim-crypto": versions.cryptoShim,
      "@deno/shim-deno": versions.denoShim,
      "@deno/shim-prompts": versions.promptsShim,
      "domexception": versions.domExceptionShim,
      "undici": versions.undici,
      "node-fetch": "~3.1.0",
    });
    const expectedText = `import { Deno } from "@deno/shim-deno";
export { Deno } from "@deno/shim-deno";
import { Blob } from "buffer";
export { Blob } from "buffer";
import { crypto } from "@deno/shim-crypto";
export { crypto, type Crypto, type SubtleCrypto, type AlgorithmIdentifier, type Algorithm, type RsaOaepParams, type BufferSource, type AesCtrParams, type AesCbcParams, type AesGcmParams, type CryptoKey, type KeyAlgorithm, type KeyType, type KeyUsage, type EcdhKeyDeriveParams, type HkdfParams, type HashAlgorithmIdentifier, type Pbkdf2Params, type AesDerivedKeyParams, type HmacImportParams, type JsonWebKey, type RsaOtherPrimesInfo, type KeyFormat, type RsaHashedKeyGenParams, type RsaKeyGenParams, type BigInteger, type EcKeyGenParams, type NamedCurve, type CryptoKeyPair, type AesKeyGenParams, type HmacKeyGenParams, type RsaHashedImportParams, type EcKeyImportParams, type AesKeyAlgorithm, type RsaPssParams, type EcdsaParams } from "@deno/shim-crypto";
import { alert, confirm, prompt } from "@deno/shim-prompts";
export { alert, confirm, prompt } from "@deno/shim-prompts";
import { default as DOMException } from "domexception";
export { default as DOMException } from "domexception";
import { File, FormData, Headers, Request, Response } from "undici";
export { File, FormData, Headers, Request, Response } from "undici";
import { default as fetch } from "node-fetch";
export { default as fetch, type RequestInit } from "node-fetch";
import { ArrayBuffer } from "./ArrayBuffer.js";
export { ArrayBuffer } from "./ArrayBuffer.js";

const dntGlobals = {
  Deno,
  Blob,
  crypto,
  alert,
  confirm,
  prompt,
  DOMException,
  File,
  FormData,
  Headers,
  Request,
  Response,
  fetch,
  ArrayBuffer,
};
export const dntGlobalThis = createMergeProxy(globalThis, dntGlobals);
`;
    assertEquals(
      output.getFileText("src/_dnt.shims.ts").substring(0, expectedText.length),
      expectedText,
    );
    output.assertExists("esm/_dnt.shims.js");
  });
});

Deno.test("should build and test polyfill project", async () => {
  await runTest("polyfill_project", {
    // also test out providing a file url for these
    entryPoints: [
      path.toFileUrl(path.resolve("./tests/polyfill_project/mod.ts"))
        .toString(),
    ],
    outDir: path.toFileUrl(path.resolve("./tests/polyfill_project/npm/"))
      .toString(),
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "polyfill-package",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertExists("esm/_dnt.polyfills.js");
  });

  await runTest("polyfill_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "polyfill-package",
      version: "1.0.0",
    },
    compilerOptions: {
      // ensure it works with the latest declarations enabled
      lib: ["ESNext", "DOM"],
    },
  }, (output) => {
    output.assertExists("esm/_dnt.polyfills.js");
  });
});

Deno.test("should build and test the promise with resolvers polyfill project", async () => {
  // this polyfill ends up alone in the polyfill file, so ensure the generated
  // file is still a module and thus type checks (see #440)
  await runTest("polyfill_promise_with_resolvers_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "polyfill-package",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertExists("esm/_dnt.polyfills.js");
  });
});

Deno.test("should build and test the array find last polyfill project", async () => {
  await runTest("polyfill_array_find_last_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "polyfill-package",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertExists("esm/_dnt.polyfills.js");
  });
});

Deno.test("should build and test the import meta polyfill project", async () => {
  await runTest("polyfill_import_meta_project", {
    test: true,
    typeCheck: "both",
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: { deno: "dev" },
    // see issue #472 -- `@types/node` declares `ImportMeta.resolve` in terms
    // of the dom's `URL`, so the polyfill's own declaration must not conflict
    compilerOptions: {
      lib: ["ES2022", "DOM"],
    },
    package: {
      name: "polyfill-import-meta-project",
      version: "0.0.0",
      devDependencies: {
        "@types/node": versions.newNodeTypes,
      },
    },
  }, (output) => {
    output.assertExists("esm/_dnt.polyfills.js");
  });
});

Deno.test("should not polyfill import.meta when disabled for an esm only build", async () => {
  await runTest("polyfill_disabled_project", {
    test: false,
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    scriptModule: false,
    shims: { deno: "dev" },
    polyfills: { importMeta: false },
    package: {
      name: "polyfill-disabled-project",
      version: "0.0.0",
    },
  }, (output) => {
    output.assertNotExists("esm/_dnt.polyfills.js");
    // the call sites should be left alone rather than rewritten to a
    // ponyfill that's no longer emitted
    assertStringIncludes(output.getFileText("esm/mod.js"), "import.meta.url");
  });
});

Deno.test("should error disabling the import.meta polyfill with a script module", async () => {
  await assertRejects(
    () =>
      runTest("polyfill_import_meta_project", {
        entryPoints: ["mod.ts"],
        outDir: "./npm",
        shims: { deno: "dev" },
        polyfills: { importMeta: false },
        package: {
          name: "polyfill-import-meta-project",
          version: "0.0.0",
        },
      }),
    Error,
    "cannot be disabled when emitting a script module",
  );
});

Deno.test("should build the polyfill project with all polyfills disabled", async () => {
  await runTest("polyfill_project", {
    test: false,
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    scriptModule: false,
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    polyfills: false,
    compilerOptions: {
      // the polyfills provide ambient declarations, so opting out of them
      // means relying on the lib declarations instead
      lib: ["ESNext"],
    },
    package: {
      name: "polyfill-package",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertNotExists("esm/_dnt.polyfills.js");
  });
});

Deno.test("should build and test the array.fromAsync polyfill project", async () => {
  await runTest("polyfill_array_from_async_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "polyfill-package",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertExists("esm/_dnt.polyfills.js");
  });
});

Deno.test("should build and test module mappings files project", async () => {
  await runTest("module_mappings_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "node-files-package",
      version: "1.0.0",
    },
    mappings: {
      "./output.deno.ts": "./output.node.ts",
    },
  }, (output) => {
    output.assertExists("esm/output.node.js");
    output.assertNotExists("esm/output.deno.js");
  });
});

Deno.test("should handle json modules", async () => {
  await runTest("json_module_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "json-module-package",
      version: "1.0.0",
    },
    compilerOptions: {
      target: "ES2015",
    },
  }, (output) => {
    output.assertNotExists("esm/data.json");
    output.assertExists("esm/data.js");
  });
});

Deno.test("should build project with another package manager", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
    packageManager: "yarn",
    typeCheck: false,
    scriptModule: false,
    declaration: false,
  }, (output) => {
    output.assertExists("yarn.lock");
    output.assertNotExists("package-lock.json");
  });
});

Deno.test("should build the import map project", async () => {
  await runTest("import_map_project", {
    entryPoints: ["mod.ts"],
    importMap: "./import_map.json",
    testPattern: "**/*_testfile.ts",
    outDir: "./npm",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
    typeCheck: "single",
  }, (_output) => {
  });
});

Deno.test("should shim web sockets", { ignore: true }, async () => {
  await runTest("web_socket_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      deno: "dev",
      webSocket: true,
    },
    package: {
      name: "server",
      version: "1.0.0",
    },
  });
});

Deno.test("should build undici project", async () => {
  await runTest("undici_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      undici: true,
    },
    package: {
      name: "undici-project",
      version: "1.0.0",
    },
  });
});

Deno.test("should build a project with a dependency that resolves to a .ts file", async () => {
  await runTest("node_modules_ts_project", {
    entryPoints: ["mod.ts", "nested/other.ts"],
    outDir: "./npm",
    scriptModule: false,
    declaration: "separate",
    test: false,
    shims: {},
    package: {
      name: "node-modules-ts-project",
      version: "1.0.0",
    },
  }, (output) => {
    // see issue #460 -- the dependency's own .ts file must not shift the
    // output down a directory, which would leave the package.json paths
    // pointing at files that don't exist
    output.assertExists("esm/mod.js");
    output.assertExists("esm/nested/other.js");
    output.assertNotExists("esm/src/mod.js");
    output.assertNotExists("esm/node_modules");
    assertEquals(output.packageJson.module, "./esm/mod.js");
    assertEquals(
      output.packageJson.exports["."].import.default,
      "./esm/mod.js",
    );
  });
});

Deno.test("should run the test preload module", async () => {
  await runTest("test_preload_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    typeCheck: "both",
    shims: {
      ...getAllShimOptions(false),
      deno: { test: "dev" },
    },
    testPreloadModule: "./scripts/test_preload.ts",
    package: {
      name: "test-preload-project",
      version: "1.0.0",
    },
  }, (output) => {
    assertPreloadModuleOutput(output);
  });
});

Deno.test("should run the test preload module when it matches the test pattern", async () => {
  await runTest("test_preload_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    typeCheck: false,
    shims: {
      ...getAllShimOptions(false),
      deno: { test: "dev" },
    },
    // this matches the preload module as well, so it should not be
    // collected as a test file twice
    testPattern: "**/{*.test.ts,test_preload.ts}",
    testPreloadModule: "./scripts/test_preload.ts",
    package: {
      name: "test-preload-project",
      version: "1.0.0",
    },
  }, (output) => {
    assertPreloadModuleOutput(output);
  });
});

Deno.test("should build and type check node types project", async () => {
  await runTest("node_types_project", {
    scriptModule: false,
    test: false,
    entryPoints: ["main.ts"],
    outDir: "./npm",
    shims: {
      // see issue 185
      custom: [{
        globalNames: ["TextEncoder", "TextDecoder"],
        module: "util",
      }],
    },
    package: {
      name: "node_types",
      version: "0.0.0",
      devDependencies: {
        "@types/node": versions.nodeTypes,
      },
    },
  });
});

Deno.test("should have the ability to ignore type checking errors", async () => {
  const foundDiagnostics: unknown[] = [];
  await runTest("node_types_project", {
    scriptModule: false,
    test: false,
    entryPoints: ["main.ts"],
    outDir: "./npm",
    shims: {
      // see issue 185
      custom: [{
        globalNames: ["TextEncoder", "TextDecoder"],
        module: "util",
      }],
    },
    package: {
      name: "node_types",
      version: "0.0.0",
    },
    filterDiagnostic(diagnostic) {
      foundDiagnostics.push(diagnostic);
      return false;
    },
  });
  assertEquals(foundDiagnostics.length, 5);
});

Deno.test("should build and type check declaration import project", async () => {
  await runTest("declaration_import_project", {
    test: false,
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {},
    package: {
      name: "declaration_project",
      version: "0.0.0",
    },
  });
});

Deno.test("using declaration project", async () => {
  await runTest("using_decl_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    shims: {
      deno: {
        test: true,
      },
    },
    compilerOptions: {
      lib: ["ESNext.Disposable"],
    },
    package: {
      name: "declaration_project",
      version: "0.0.0",
    },
  });
});

Deno.test("should build jsr project", async () => {
  await runTest("jsr_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    typeCheck: "both",
    shims: {
      ...getAllShimOptions(false),
      deno: "dev",
      weakRef: true,
    },
    package: {
      name: "add",
      version: "1.0.0",
    },
    compilerOptions: {
      importHelpers: true,
    },
  }, (output) => {
    output.assertNotExists("script/mod.js.map");
    output.assertNotExists("esm/mod.js.map");
    assertEquals(output.packageJson, {
      name: "add",
      version: "1.0.0",
      main: "./script/mod.js",
      module: "./esm/mod.js",
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: "./script/mod.js",
        },
      },
      scripts: {
        test: "node test_runner.cjs",
      },
      dependencies: {
        tslib: versions.tsLib,
        "@deno/sham-weakref": versions.weakRefSham,
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
        picocolors: versions.picocolors,
        "@deno/shim-deno": versions.denoShim,
      },
      _generatedBy: "dnt@dev",
    });
    assertEquals(
      output.npmIgnore,
      `/src/
/esm/mod.test.js
/esm/mod.test.d.ts
/script/mod.test.js
/script/mod.test.d.ts
/esm/_dnt.test_shims.js
/esm/_dnt.test_shims.d.ts
/script/_dnt.test_shims.js
/script/_dnt.test_shims.d.ts
/test_runner.cjs
yarn.lock
pnpm-lock.yaml
`,
    );
  });
});

Deno.test("should build workspace project", async () => {
  for (
    const configFile of [
      import.meta.resolve("./workspace_project/deno.json"),
      undefined,
    ]
  ) {
    await runTest("workspace_project", {
      entryPoints: ["./add/mod.ts"],
      outDir: "./npm",
      typeCheck: "both",
      shims: {
        ...getAllShimOptions(false),
      },
      configFile,
      package: {
        name: "add",
        version: "1.0.0",
      },
      compilerOptions: {
        lib: ["ESNext", "DOM"],
        importHelpers: true,
      },
    }, (output) => {
      output.assertNotExists("script/mod.js.map");
      output.assertNotExists("esm/mod.js.map");
      assertEquals(output.packageJson, {
        name: "add",
        version: "1.0.0",
        main: "./script/mod.js",
        module: "./esm/mod.js",
        exports: {
          ".": {
            import: "./esm/mod.js",
            require: "./script/mod.js",
          },
        },
        scripts: {
          test: "node test_runner.cjs",
        },
        dependencies: {
          tslib: versions.tsLib,
        },
        devDependencies: {
          picocolors: versions.picocolors,
        },
        _generatedBy: "dnt@dev",
      });
      assertEquals(
        output.npmIgnore,
        `/src/
/test_runner.cjs
yarn.lock
pnpm-lock.yaml
`,
      );
    });
  }
});

Deno.test("should error building with a frozen and out of date lock file", async () => {
  const error = await assertRejects(() =>
    runTest("frozen_lockfile_project", {
      entryPoints: ["mod.ts"],
      outDir: "./npm",
      frozenLockfile: true,
      typeCheck: false,
      test: false,
      skipNpmInstall: true,
      shims: {},
      package: {
        name: "add",
        version: "1.0.0",
      },
    })
  );
  assertStringIncludes(String(error), "The lockfile is out of date.");
});

Deno.test("should build with an out of date lock file when not frozen", async () => {
  await runTest("frozen_lockfile_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    typeCheck: false,
    test: false,
    skipNpmInstall: true,
    shims: {},
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    output.assertExists("esm/mod.js");
  });
});

export interface Output {
  packageJson: any;
  npmIgnore: string;
  getFileText(filePath: string): string;
  assertExists(filePath: string): void;
  assertNotExists(filePath: string): void;
}

async function runTest(
  project:
    | "bin_shebang_project"
    | "declaration_import_project"
    | "frozen_lockfile_project"
    | "import_map_project"
    | "json_module_project"
    | "jsr_project"
    | "package_mappings_project"
    | "polyfill_project"
    | "polyfill_array_from_async_project"
    | "polyfill_array_find_last_project"
    | "polyfill_disabled_project"
    | "polyfill_promise_with_resolvers_project"
    | "polyfill_import_meta_project"
    | "module_mappings_project"
    | "node_modules_ts_project"
    | "node_types_project"
    | "undici_project"
    | "shim_project"
    | "test_preload_project"
    | "test_project"
    | "tla_project"
    | "web_socket_project"
    | "using_decl_project"
    | "workspace_project",
  options: BuildOptions,
  checkOutput?: (output: Output) => Promise<void> | void,
) {
  const originalCwd = Deno.cwd();
  const outDirPath = options.outDir.startsWith("file:")
    ? path.fromFileUrl(options.outDir)
    : options.outDir;
  Deno.chdir(`./tests/${project}`);
  tryRemoveOutDir();
  try {
    await build(options);
    const getFileText = (filePath: string) => {
      return Deno.readTextFileSync(outDirPath + "/" + filePath);
    };
    if (checkOutput) {
      const packageJson = JSON.parse(getFileText("package.json"));
      const npmIgnore = getFileText(".npmignore");
      await checkOutput({
        packageJson,
        npmIgnore,
        getFileText,
        assertExists(filePath) {
          Deno.statSync("npm/" + filePath);
        },
        assertNotExists(filePath) {
          try {
            Deno.statSync("npm/" + filePath);
            throw new Error(`Found file at ${filePath}`);
          } catch (err) {
            if (!(err instanceof Deno.errors.NotFound)) {
              throw err;
            }
          }
        },
      });
    }
  } finally {
    tryRemoveOutDir();
    Deno.chdir(originalCwd);
  }

  function tryRemoveOutDir() {
    try {
      Deno.removeSync(outDirPath, { recursive: true });
    } catch (err) {
      if (!(err instanceof Deno.errors.NotFound)) {
        console.error(`Error removing dir: ${err}`);
      }
    }
  }
}

function assertPreloadModuleOutput(output: Output) {
  // the preload module should be emitted, but not distributed
  output.assertExists("esm/scripts/test_preload.js");
  output.assertExists("script/scripts/test_preload.js");
  assertStringIncludes(output.npmIgnore, "/esm/scripts/test_preload.js\n");
  assertStringIncludes(output.npmIgnore, "/script/scripts/test_preload.js\n");

  // it should be loaded once for each output, before any test file
  const testRunnerText = output.getFileText("test_runner.cjs");
  assertStringIncludes(
    testRunnerText,
    `require("./script/scripts/test_preload.js");`,
  );
  assertStringIncludes(
    testRunnerText,
    `await import("./esm/scripts/test_preload.js");`,
  );

  // it should not be run as a test file
  const filePaths = /const filePaths = \[([^\]]*)\]/.exec(testRunnerText)![1];
  assertStringIncludes(filePaths, "mod.test.js");
  assertEquals(filePaths.includes("test_preload"), false);
}

function getAllShimOptions(value: ShimValue): ShimOptions {
  return {
    deno: value,
    prompts: value,
    blob: value,
    crypto: value,
    domException: value,
    undici: value,
  };
}
