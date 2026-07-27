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
  const fileIndexArg = process.argv[2];
  if (fileIndexArg == null) {
    const { spawnSync } = require("child_process");
    let failed = false;
    for (const [i, _] of filePaths.entries()) {
      if (i > 0) {
        console.log("");
      }
      const result = spawnSync(process.execPath, [__filename, String(i)], {
        stdio: "inherit",
      });
      if (result.status !== 0) {
        failed = true;
      }
    }
    if (failed) {
      process.exit(1);
    }
    return;
  }

  const filePath = filePaths[Number(fileIndexArg)];

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
  const fileIndexArg = process.argv[2];
  if (fileIndexArg == null) {
    const { spawnSync } = require("child_process");
    let failed = false;
    for (const [i, _] of filePaths.entries()) {
      if (i > 0) {
        console.log("");
      }
      const result = spawnSync(process.execPath, [__filename, String(i)], {
        stdio: "inherit",
      });
      if (result.status !== 0) {
        failed = true;
      }
    }
    if (failed) {
      process.exit(1);
    }
    return;
  }

  const filePath = filePaths[Number(fileIndexArg)];

  const testContext = {
    process,
    pc,
  };
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

async function runTestDefinitions(testDefinitions, options) {
  const testFailures = [];
  const hasOnly = testDefinitions.some((d)=>d.only);
  if (hasOnly) {
    testDefinitions = testDefinitions.filter((d)=>d.only);
  }
  for (const definition of testDefinitions){
    options.process.stdout.write("test " + definition.name + " ...");
    if (definition.ignore) {
      options.process.stdout.write(\` \${options.pc.gray("ignored")}\\n\`);
      continue;
    }
    const context = getTestContext(definition, undefined);
    let pass = false;
    try {
      await definition.fn(context);
      if (context.hasFailingChild) {
        testFailures.push({
          name: definition.name,
          err: new Error("Had failing test step.")
        });
      } else {
        pass = true;
      }
    } catch (err) {
      testFailures.push({
        name: definition.name,
        err
      });
    }
    const testStepOutput = context.getOutput();
    if (testStepOutput.length > 0) {
      options.process.stdout.write(testStepOutput);
    } else {
      options.process.stdout.write(" ");
    }
    options.process.stdout.write(getStatusText(pass ? "ok" : "fail"));
    options.process.stdout.write("\\n");
  }
  if (testFailures.length > 0) {
    options.process.stdout.write("\\nFAILURES");
    for (const failure of testFailures){
      options.process.stdout.write("\\n\\n");
      options.process.stdout.write(failure.name + "\\n");
      options.process.stdout.write(indentText((failure.err?.stack ?? failure.err).toString(), 1));
    }
    options.process.exit(1);
  } else if (hasOnly) {
    options.process.stdout.write('error: Test failed because the "only" option was used.\\n');
    options.process.exit(1);
  }
  function getTestContext(definition, parent) {
    return {
      name: definition.name,
      parent,
      origin: options.origin,
      /** @type {any} */ err: undefined,
      status: "ok",
      children: [],
      get hasFailingChild () {
        return this.children.some((c)=>c.status === "fail" || c.status === "pending");
      },
      getOutput () {
        let output = "";
        if (this.parent) {
          output += "test " + this.name + " ...";
        }
        if (this.children.length > 0) {
          output += "\\n" + this.children.map((c)=>indentText(c.getOutput(), 1)).join("\\n") + "\\n";
        } else if (!this.err) {
          output += " ";
        }
        if (this.parent && this.err) {
          output += "\\n";
        }
        if (this.err) {
          output += indentText((this.err.stack ?? this.err).toString(), 1);
          if (this.parent) {
            output += "\\n";
          }
        }
        if (this.parent) {
          output += getStatusText(this.status);
        }
        return output;
      },
      async step (nameOrTestDefinition, fn) {
        const definition = getDefinition();
        const context = getTestContext(definition, this);
        context.status = "pending";
        this.children.push(context);
        if (definition.ignore) {
          context.status = "ignored";
          return false;
        }
        try {
          await definition.fn(context);
          context.status = "ok";
          if (context.hasFailingChild) {
            context.status = "fail";
            return false;
          }
          return true;
        } catch (err) {
          context.status = "fail";
          context.err = err;
          return false;
        }
        /** @returns {TestDefinition} */ function getDefinition() {
          if (typeof nameOrTestDefinition === "string") {
            if (!(fn instanceof Function)) {
              throw new TypeError("Expected function for second argument.");
            }
            return {
              name: nameOrTestDefinition,
              fn
            };
          } else if (typeof nameOrTestDefinition === "object") {
            return nameOrTestDefinition;
          } else {
            throw new TypeError("Expected a test definition or name and function.");
          }
        }
      }
    };
  }
  function getStatusText(status) {
    switch(status){
      case "ok":
        return options.pc.green(status);
      case "fail":
      case "pending":
        return options.pc.red(status);
      case "ignored":
        return options.pc.gray(status);
      default:
        {
          const _assertNever = status;
          return status;
        }
    }
  }
  function indentText(text, indentLevel) {
    if (text === undefined) {
      text = "[undefined]";
    } else if (text === null) {
      text = "[null]";
    } else {
      text = text.toString();
    }
    return text.split(/\\r?\\n/).map((line)=>"  ".repeat(indentLevel) + line).join("\\n");
  }
}

main();
`,
  );
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
  const fileIndexArg = process.argv[2];
  if (fileIndexArg == null) {
    const { spawnSync } = require("child_process");
    let failed = false;
    for (const [i, _] of filePaths.entries()) {
      if (i > 0) {
        console.log("");
      }
      const result = spawnSync(process.execPath, [__filename, String(i)], {
        stdio: "inherit",
      });
      if (result.status !== 0) {
        failed = true;
      }
    }
    if (failed) {
      process.exit(1);
    }
    return;
  }

  const filePath = filePaths[Number(fileIndexArg)];

  process.chdir(__dirname + "/script");
  try {
    require("./script/scripts/test_preload.js");
  } catch(err) {
    console.error(err);
    process.exit(1);
  }
  process.chdir(__dirname + "/esm");
  await import("./esm/scripts/test_preload.js");

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
  const fileIndexArg = process.argv[2];
  if (fileIndexArg == null) {
    const { spawnSync } = require("child_process");
    let failed = false;
    for (const [i, _] of filePaths.entries()) {
      if (i > 0) {
        console.log("");
      }
      const result = spawnSync(process.execPath, [__filename, String(i)], {
        stdio: "inherit",
      });
      if (result.status !== 0) {
        failed = true;
      }
    }
    if (failed) {
      process.exit(1);
    }
    return;
  }

  const filePath = filePaths[Number(fileIndexArg)];

  process.chdir(__dirname + "/esm");
  await import("./esm/test_preload.js");

  const esmPath = "./esm/" + filePath;
  console.log("\\nRunning tests in " + pc.underline(esmPath) + "...\\n");
  process.chdir(__dirname + "/esm");
  await import(esmPath);
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
  const fileIndexArg = process.argv[2];
  if (fileIndexArg == null) {
    const { spawnSync } = require("child_process");
    let failed = false;
    for (const [i, _] of filePaths.entries()) {
      if (i > 0) {
        console.log("");
      }
      const result = spawnSync(process.execPath, [__filename, String(i)], {
        stdio: "inherit",
      });
      if (result.status !== 0) {
        failed = true;
      }
    }
    if (failed) {
      process.exit(1);
    }
    return;
  }

  const filePath = filePaths[Number(fileIndexArg)];

  process.chdir(__dirname + "/script");
  try {
    require("./script/test_preload.js");
  } catch(err) {
    console.error(err);
    process.exit(1);
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
  const fileIndexArg = process.argv[2];
  if (fileIndexArg == null) {
    const { spawnSync } = require("child_process");
    let failed = false;
    for (const [i, _] of filePaths.entries()) {
      if (i > 0) {
        console.log("");
      }
      const result = spawnSync(process.execPath, [__filename, String(i)], {
        stdio: "inherit",
      });
      if (result.status !== 0) {
        failed = true;
      }
    }
    if (failed) {
      process.exit(1);
    }
    return;
  }

  const filePath = filePaths[Number(fileIndexArg)];

  const esmPath = "./esm/" + filePath;
  console.log("\\nRunning tests in " + pc.underline(esmPath) + "...\\n");
  process.chdir(__dirname + "/esm");
  await import(esmPath);
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
  const fileIndexArg = process.argv[2];
  if (fileIndexArg == null) {
    const { spawnSync } = require("child_process");
    let failed = false;
    for (const [i, _] of filePaths.entries()) {
      if (i > 0) {
        console.log("");
      }
      const result = spawnSync(process.execPath, [__filename, String(i)], {
        stdio: "inherit",
      });
      if (result.status !== 0) {
        failed = true;
      }
    }
    if (failed) {
      process.exit(1);
    }
    return;
  }

  const filePath = filePaths[Number(fileIndexArg)];

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

main();
`,
  );
});
