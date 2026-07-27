// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals, assertStringIncludes } from "@std/assert";
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

Deno.test("gets code when isolating the test files", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    includeEsModule: true,
    includeScriptModule: true,
    testIsolation: "process",
  });
  assertStringIncludes(
    code,
    `  const fileIndexArg = process.argv[2];
  if (fileIndexArg == null) {
    const { spawnSync } = require("child_process");
    let failed = false;
    for (const i of filePaths.keys()) {
      if (i > 0) {
        console.log("");
      }
      const args = [...process.execArgv, __filename, String(i)];
      const result = spawnSync(process.execPath, args, { stdio: "inherit" });
      if (result.error != null) {
        console.error(result.error);
      }
      if (result.status !== 0) {
        failed = true;
      }
    }
    if (failed) {
      process.exitCode = 1;
    }
    return;
  }

  const filePath = filePaths[Number(fileIndexArg)];`,
  );
  // there's no loop over the files because each one runs in its own process
  assertEquals(code.includes("for (const [i, filePath]"), false);
});

Deno.test("gets code when a preload module is used", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    preloadEntryPoint: "scripts/test_preload.ts",
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
  process.chdir(__dirname + "/script");
  try {
    require("./script/scripts/test_preload.js");
  } catch(err) {
    console.error(err);
    process.exit(1);
  }
  process.chdir(__dirname + "/esm");
  await import("./esm/scripts/test_preload.js");

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

Deno.test("gets code when a preload module is used without cjs", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    preloadEntryPoint: "test_preload.ts",
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
  process.chdir(__dirname + "/esm");
  await import("./esm/test_preload.js");

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

Deno.test("gets code when a preload module is used without esm", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    preloadEntryPoint: "test_preload.ts",
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
  process.chdir(__dirname + "/script");
  try {
    require("./script/test_preload.js");
  } catch(err) {
    console.error(err);
    process.exit(1);
  }

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

Deno.test("gets code for jsx and tsx entry points", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.tsx", "./other.jsx", "./final.js"],
    denoTestShimPackageName: undefined,
    preloadEntryPoint: "test_preload.tsx",
    includeEsModule: true,
    includeScriptModule: false,
  });
  assertStringIncludes(
    code,
    `const filePaths = [
  "./test.js",
  "./other.js",
  "./final.js",
];`,
  );
  assertStringIncludes(code, `await import("./esm/test_preload.js");`);
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
