// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals, assertRejects, assertStringIncludes } from "@std/assert";
import * as path from "@std/path";
import { ShimValue } from "../lib/shims.ts";
import { build, BuildOptions, ShimOptions } from "../mod.ts";

const versions = {
  denoShim: "~0.18.0",
  denoTestShim: "~0.5.0",
  cryptoShim: "~0.3.1",
  domExceptionShim: "^4.0.0",
  domExceptionShimTypes: "^4.0.0",
  promptsShim: "~0.1.0",
  timersShim: "~0.1.0",
  weakRefSham: "~0.1.0",
  undici: "^6.0.0",
  picocolors: "^1.0.0",
  nodeTypes: "^20.9.0",
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
        test: "node test_runner.js",
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
/esm/_dnt.test_shims.js
/esm/_dnt.test_shims.d.ts
/script/_dnt.test_shims.js
/script/_dnt.test_shims.d.ts
/test_runner.js
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
        test: "node test_runner.js",
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
/script/_dnt.test_shims.js
/types/_dnt.test_shims.d.ts
/test_runner.js
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
      _generatedBy: "dnt@dev",
    });

    output.assertNotExists("script/mod.js");
    output.assertNotExists("types/mod.js");

    // This doesn't include the test files because they're not analyzed for in this scenario.
    assertEquals(
      output.npmIgnore,
      `/src/
/test_runner.js
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
          test: "node test_runner.js",
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

Deno.test("should build test project with declaration maps by default", async () => {
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
    output.assertNotExists("types/mod.d.ts");
    output.assertExists("script/mod.d.ts.map");
    output.assertExists("esm/mod.d.ts.map");
  });

  await runTest("test_project", {
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    declaration: "separate",
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
        test: "node test_runner.js",
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
        test: "node test_runner.js",
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
      `/esm/mod.test.js
/esm/mod.test.js.map
/esm/mod.test.d.ts
/script/mod.test.js
/script/mod.test.js.map
/script/mod.test.d.ts
/esm/deps/deno.land/std@0.181.0/fmt/colors.js
/esm/deps/deno.land/std@0.181.0/fmt/colors.js.map
/esm/deps/deno.land/std@0.181.0/fmt/colors.d.ts
/script/deps/deno.land/std@0.181.0/fmt/colors.js
/script/deps/deno.land/std@0.181.0/fmt/colors.js.map
/script/deps/deno.land/std@0.181.0/fmt/colors.d.ts
/esm/deps/deno.land/std@0.181.0/testing/_diff.js
/esm/deps/deno.land/std@0.181.0/testing/_diff.js.map
/esm/deps/deno.land/std@0.181.0/testing/_diff.d.ts
/script/deps/deno.land/std@0.181.0/testing/_diff.js
/script/deps/deno.land/std@0.181.0/testing/_diff.js.map
/script/deps/deno.land/std@0.181.0/testing/_diff.d.ts
/esm/deps/deno.land/std@0.181.0/testing/_format.js
/esm/deps/deno.land/std@0.181.0/testing/_format.js.map
/esm/deps/deno.land/std@0.181.0/testing/_format.d.ts
/script/deps/deno.land/std@0.181.0/testing/_format.js
/script/deps/deno.land/std@0.181.0/testing/_format.js.map
/script/deps/deno.land/std@0.181.0/testing/_format.d.ts
/esm/deps/deno.land/std@0.181.0/testing/asserts.js
/esm/deps/deno.land/std@0.181.0/testing/asserts.js.map
/esm/deps/deno.land/std@0.181.0/testing/asserts.d.ts
/script/deps/deno.land/std@0.181.0/testing/asserts.js
/script/deps/deno.land/std@0.181.0/testing/asserts.js.map
/script/deps/deno.land/std@0.181.0/testing/asserts.d.ts
/esm/_dnt.test_shims.js
/esm/_dnt.test_shims.js.map
/esm/_dnt.test_shims.d.ts
/script/_dnt.test_shims.js
/script/_dnt.test_shims.js.map
/script/_dnt.test_shims.d.ts
/test_runner.js
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
        test: "node test_runner.js",
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
/test_runner.js
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
        test: "node test_runner.js",
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
      "@deno/shim-timers": versions.timersShim,
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
      "@deno/shim-timers": versions.timersShim,
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
import { setInterval, setTimeout } from "@deno/shim-timers";
export { setInterval, setTimeout } from "@deno/shim-timers";
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
  setInterval,
  setTimeout,
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
        test: "node test_runner.js",
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
/test_runner.js
yarn.lock
pnpm-lock.yaml
`,
    );
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
    | "declaration_import_project"
    | "import_map_project"
    | "json_module_project"
    | "jsr_project"
    | "package_mappings_project"
    | "polyfill_project"
    | "polyfill_array_from_async_project"
    | "polyfill_array_find_last_project"
    | "module_mappings_project"
    | "node_types_project"
    | "undici_project"
    | "shim_project"
    | "test_project"
    | "tla_project"
    | "web_socket_project"
    | "using_decl_project",
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

function getAllShimOptions(value: ShimValue): ShimOptions {
  return {
    deno: value,
    timers: value,
    prompts: value,
    blob: value,
    crypto: value,
    domException: value,
    undici: value,
  };
}
