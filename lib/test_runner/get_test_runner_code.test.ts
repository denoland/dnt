// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "../test.deps.ts";
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
    `const chalk = require("chalk");
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
    console.log("Running tests in " + chalk.underline(scriptPath) + "...\\n");
    process.chdir(__dirname + "/script");
    try {
      require(scriptPath);
    } catch(err) {
      console.error(err);
      process.exit(1);
    }

    const esmPath = "./esm/" + filePath;
    console.log("\\nRunning tests in " + chalk.underline(esmPath) + "...\\n");
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
    `const chalk = require("chalk");
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
    chalk,
  };
  for (const [i, filePath] of filePaths.entries()) {
    if (i > 0) {
      console.log("");
    }

    const scriptPath = "./script/" + filePath;
    console.log("Running tests in " + chalk.underline(scriptPath) + "...\\n");
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
    console.log("\\nRunning tests in " + chalk.underline(esmPath) + "...\\n");
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

Deno.test("gets code when cjs is not used", () => {
  const code = getTestRunnerCode({
    testEntryPoints: ["./test.ts"],
    denoTestShimPackageName: undefined,
    includeEsModule: true,
    includeScriptModule: false,
  });
  assertEquals(
    code,
    `const chalk = require("chalk");
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
    console.log("\\nRunning tests in " + chalk.underline(esmPath) + "...\\n");
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
  console.log(code);
  assertEquals(
    code,
    `const chalk = require("chalk");
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
    console.log("Running tests in " + chalk.underline(scriptPath) + "...\\n");
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
