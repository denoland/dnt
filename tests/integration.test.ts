// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import {
  assertEquals,
  assertRejects,
  assertStringIncludes,
} from "https://deno.land/std@0.128.0/testing/asserts.ts";
import { build, BuildOptions, ShimOptions } from "../mod.ts";

const versions = {
  denoShim: "~0.4.3",
  denoTestShim: "~0.3.2",
  cryptoShim: "~0.2.0",
  domExceptionShim: "^4.0.0",
  domExceptionShimTypes: "^2.0.1",
  promptsShim: "~0.1.0",
  timersShim: "~0.1.0",
  weakRefSham: "~0.1.0",
  undici: "^4.12.1",
  chalk: "4.1.2",
  nodeTypes: "16.11.26",
  tsLib: "2.3.1",
};

Deno.test("should build test project", async () => {
  await runTest("test_project", {
    entryPoints: ["mod.ts"],
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
      module: "./esm/mod.js",
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: "./script/mod.js",
          types: "./types/mod.d.ts",
        },
      },
      scripts: {
        test: "node test_runner.js",
      },
      types: "./types/mod.d.ts",
      dependencies: {
        tslib: versions.tsLib,
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
        chalk: versions.chalk,
        "@deno/shim-deno": versions.denoShim,
        "@deno/sham-weakref": versions.weakRefSham,
      },
    });
    assertEquals(
      output.npmIgnore,
      `src/
esm/mod.test.js
script/mod.test.js
types/mod.test.d.ts
esm/deps/deno.land/std@0.128.0/fmt/colors.js
script/deps/deno.land/std@0.128.0/fmt/colors.js
types/deps/deno.land/std@0.128.0/fmt/colors.d.ts
esm/deps/deno.land/std@0.128.0/testing/_diff.js
script/deps/deno.land/std@0.128.0/testing/_diff.js
types/deps/deno.land/std@0.128.0/testing/_diff.d.ts
esm/deps/deno.land/std@0.128.0/testing/asserts.js
script/deps/deno.land/std@0.128.0/testing/asserts.js
types/deps/deno.land/std@0.128.0/testing/asserts.d.ts
esm/_dnt.test_shims.js
script/_dnt.test_shims.js
types/_dnt.test_shims.d.ts
test_runner.js
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
      deno: "dev",
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
      dependencies: {},
      devDependencies: {},
    });

    output.assertNotExists("script/mod.js");
    output.assertNotExists("types/mod.js");

    // This doesn't include the test files because they're not analyzed for in this scenario.
    assertEquals(
      output.npmIgnore,
      `src/
test_runner.js
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
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    const fileText = output.getFileText("script/mod.js");
    assertStringIncludes(fileText, "(function (factory) {");
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
      dependencies: {},
      devDependencies: {
        "@types/node": versions.nodeTypes,
        chalk: versions.chalk,
        "@deno/shim-deno": versions.denoShim,
      },
      exports: {},
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
      chalk: versions.chalk,
      "@deno/shim-deno-test": versions.denoTestShim,
      "@deno/sham-weakref": versions.weakRefSham,
    });
  });
});

Deno.test("error for TLA when emitting CommonJS", async () => {
  await assertRejects(() =>
    runTest("tla_project", {
      entryPoints: ["mod.ts"],
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
          import: "./esm/mod.js",
          types: "./types/mod.d.ts",
        },
      },
      scripts: {
        test: "node test_runner.js",
      },
      types: "./types/mod.d.ts",
      dependencies: {},
      devDependencies: {
        "@types/node": versions.nodeTypes,
        chalk: versions.chalk,
        "@deno/shim-deno": versions.denoShim,
      },
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
      `esm/mod.test.js
script/mod.test.js
types/mod.test.d.ts
esm/deps/deno.land/std@0.128.0/fmt/colors.js
script/deps/deno.land/std@0.128.0/fmt/colors.js
types/deps/deno.land/std@0.128.0/fmt/colors.d.ts
esm/deps/deno.land/std@0.128.0/testing/_diff.js
script/deps/deno.land/std@0.128.0/testing/_diff.js
types/deps/deno.land/std@0.128.0/testing/_diff.d.ts
esm/deps/deno.land/std@0.128.0/testing/asserts.js
script/deps/deno.land/std@0.128.0/testing/asserts.js
types/deps/deno.land/std@0.128.0/testing/asserts.d.ts
esm/_dnt.test_shims.js
script/_dnt.test_shims.js
types/_dnt.test_shims.d.ts
test_runner.js
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
        version: "^11.0.0",
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
          import: "./esm/mod.js",
          require: "./script/mod.js",
          types: "./types/mod.d.ts",
        },
      },
      scripts: {
        test: "node test_runner.js",
      },
      types: "./types/mod.d.ts",
      dependencies: {
        "code-block-writer": "^11.0.0",
      },
      devDependencies: {
        "@types/node": versions.nodeTypes,
        chalk: versions.chalk,
        "@deno/shim-deno": versions.denoShim,
      },
    });
    assertEquals(
      output.npmIgnore,
      `src/
esm/mod.test.js
script/mod.test.js
types/mod.test.d.ts
esm/_dnt.test_shims.js
script/_dnt.test_shims.js
types/_dnt.test_shims.d.ts
test_runner.js
yarn.lock
pnpm-lock.yaml
`,
    );
  });
});

Deno.test("should build shim project", async () => {
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
      "chalk": versions.chalk,
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

Deno.test("should build polyfill project", async () => {
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
    typeCheck: true,
  }, (_output) => {
  });
});

Deno.test("should shim web sockets", async () => {
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

export interface Output {
  packageJson: any;
  npmIgnore: string;
  getFileText(filePath: string): string;
  assertExists(filePath: string): void;
  assertNotExists(filePath: string): void;
}

async function runTest(
  project:
    | "import_map_project"
    | "json_module_project"
    | "package_mappings_project"
    | "polyfill_project"
    | "module_mappings_project"
    | "shim_project"
    | "test_project"
    | "tla_project"
    | "web_socket_project",
  options: BuildOptions,
  checkOutput?: (output: Output) => (Promise<void> | void),
) {
  const originalCwd = Deno.cwd();
  Deno.chdir(`./tests/${project}`);
  try {
    await build(options);
    const getFileText = (filePath: string) => {
      return Deno.readTextFileSync(options.outDir + "/" + filePath);
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
    try {
      Deno.removeSync(options.outDir, { recursive: true });
    } catch (err) {
      console.error(`Error removing dir: ${err}`);
    } finally {
      Deno.chdir(originalCwd);
    }
  }
}

function getAllShimOptions(value: boolean | "dev"): ShimOptions {
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
