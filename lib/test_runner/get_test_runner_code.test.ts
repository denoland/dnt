// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals } from "@std/assert";
import { getTestRunnerCode } from "./get_test_runner_code.ts";
import { runTestDefinitions } from "./test_runner.ts";

const runTestDefinitionsCode = runTestDefinitions.toString()
  .replace("export async function", "async function");

Deno.test("gets code when no shim used", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    includeEsModule: true,
    includeScriptModule: true,
  });
  assertEquals(
    code,
    `const pc = require("picocolors");
const process = require("process");

const filePaths = [
  "./test.js",
];

async function main() {
  for (const [i, filePath] of filePaths.entries()) {
    if (i > 0) {
      console.log("");
    }

    const scriptPath = "./script/" + filePath;
    console.log("Running tests in " + pc.underline(scriptPath) + "...\\n");
    process.chdir(__dirname + "/script");
    try {
      require(scriptPath);
    } catch(err) {
      console.error(err);
      process.exit(1);
    }

    const esmPath = "./esm/" + filePath;
    console.log("\\nRunning tests in " + pc.underline(esmPath) + "...\\n");
    process.chdir(__dirname + "/esm");
    await import(esmPath);
  }
}

main();
`,
  );
});

Deno.test("gets code when shim used", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./1.test.ts", "./2.test.ts"],
    denoTestShimPackageName: "test-shim-package/test-internals",
    includeEsModule: true,
    includeScriptModule: true,
  });
  assertEquals(
    code,
    `const pc = require("picocolors");
const process = require("process");
const { pathToFileURL } = require("url");
const { testDefinitions } = require("test-shim-package/test-internals");

const filePaths = [
  "./1.test.js",
  "./2.test.js",
];

async function main() {
  const testContext = {
    process,
    pc,
  };
  for (const [i, filePath] of filePaths.entries()) {
    if (i > 0) {
      console.log("");
    }

    const scriptPath = "./script/" + filePath;
    console.log("Running tests in " + pc.underline(scriptPath) + "...\\n");
    process.chdir(__dirname + "/script");
    const scriptTestContext = {
      origin: pathToFileURL(filePath).toString(),
      ...testContext,
    };
    try {
      require(scriptPath);
    } catch(err) {
      console.error(err);
      process.exit(1);
    }
    await runTestDefinitions(testDefinitions.splice(0, testDefinitions.length), scriptTestContext);

    const esmPath = "./esm/" + filePath;
    console.log("\\nRunning tests in " + pc.underline(esmPath) + "...\\n");
    process.chdir(__dirname + "/esm");
    const esmTestContext = {
      origin: pathToFileURL(filePath).toString(),
      ...testContext,
    };
    await import(esmPath);
    await runTestDefinitions(testDefinitions.splice(0, testDefinitions.length), esmTestContext);
  }
}

${runTestDefinitionsCode}

main();
`,
  );
});

Deno.test("gets code that sets the globals of the test shims", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    includeEsModule: true,
    includeScriptModule: false,
    testShims: [{
      // the `Deno` namespace is never set as a global, so this
      // shim should be ignored
      package: {
        name: "@deno/shim-deno",
        version: "~0.18.0",
      },
      globalNames: ["Deno"],
    }, {
      package: {
        name: "test-shim-package",
        version: "^1.0.0",
        subPath: "sub-path",
      },
      globalNames: ["Headers", {
        name: "DOMException",
        exportName: "default",
      }, {
        name: "RequestInit",
        typeOnly: true,
      }],
    }, {
      module: "node:buffer",
      globalNames: ["Blob"],
    }, {
      // only has type only globals, so it should be ignored
      module: "node:util",
      globalNames: [{
        name: "SomeType",
        typeOnly: true,
      }],
    }],
  });
  assertEquals(
    code,
    `const pc = require("picocolors");
const process = require("process");

const filePaths = [
  "./test.js",
];

async function main() {
  await setUpGlobals();
  for (const [i, filePath] of filePaths.entries()) {
    if (i > 0) {
      console.log("");
    }

    const esmPath = "./esm/" + filePath;
    console.log("\\nRunning tests in " + pc.underline(esmPath) + "...\\n");
    process.chdir(__dirname + "/esm");
    await import(esmPath);
  }
}

async function setUpGlobals() {
  const shim0 = await import("test-shim-package/sub-path");
  defineGlobal("Headers", shim0.Headers);
  defineGlobal("DOMException", shim0.default);
  const shim1 = await import("node:buffer");
  defineGlobal("Blob", shim1.Blob);

  function defineGlobal(name, value) {
    // some globals are defined as getters on globalThis (ex. \`crypto\`),
    // so a plain assignment would not work here
    Object.defineProperty(globalThis, name, {
      value,
      writable: true,
      enumerable: false,
      configurable: true,
    });
  }
}

main();
`,
  );
});

Deno.test("does not set the globals of local and remote shims", () => {
  // these modules are in the graph, so the test runner has no way of
  // resolving them to an output file
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    includeEsModule: true,
    includeScriptModule: true,
    testShims: [{
      module: "./my_shim.ts",
      globalNames: ["fetch"],
    }, {
      module: "my_shim.ts",
      globalNames: ["Response"],
    }, {
      module: "https://deno.land/x/some_shim/mod.ts",
      globalNames: ["setTimeout"],
    }],
  });
  assertEquals(code.includes("setUpGlobals"), false);
});

Deno.test("gets code when cjs is not used", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    includeEsModule: true,
    includeScriptModule: false,
  });
  assertEquals(
    code,
    `const pc = require("picocolors");
const process = require("process");

const filePaths = [
  "./test.js",
];

async function main() {
  for (const [i, filePath] of filePaths.entries()) {
    if (i > 0) {
      console.log("");
    }

    const esmPath = "./esm/" + filePath;
    console.log("\\nRunning tests in " + pc.underline(esmPath) + "...\\n");
    process.chdir(__dirname + "/esm");
    await import(esmPath);
  }
}

main();
`,
  );
});

Deno.test("gets code when esm is not used", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    includeEsModule: false,
    includeScriptModule: true,
  });
  assertEquals(
    code,
    `const pc = require("picocolors");
const process = require("process");

const filePaths = [
  "./test.js",
];

async function main() {
  for (const [i, filePath] of filePaths.entries()) {
    if (i > 0) {
      console.log("");
    }

    const scriptPath = "./script/" + filePath;
    console.log("Running tests in " + pc.underline(scriptPath) + "...\\n");
    process.chdir(__dirname + "/script");
    try {
      require(scriptPath);
    } catch(err) {
      console.error(err);
      process.exit(1);
    }
  }
}

main();
`,
  );
});
