// Copyright 2018-2024 the Deno authors. MIT license.

import CodeBlockWriter from "code-block-writer";
import { toJsFilePath } from "../utils.ts";
import { runTestDefinitions } from "./test_runner.ts";

export function getTestRunnerCode(options: {
  testEntryPoints: string[];
  denoTestShimPackageName: string | undefined;
  includeEsModule: boolean | undefined;
  includeScriptModule: boolean | undefined;
  preloadEntryPoint?: string;
  testIsolation?: "process" | "none";
}) {
  const usesDenoTest = options.denoTestShimPackageName != null;
  const preloadPath = options.preloadEntryPoint == null
    ? undefined
    : toJsFilePath(options.preloadEntryPoint);
  const writer = createWriter();
  writer.writeLine(`const pc = require("picocolors");`)
    .writeLine(`const process = require("process");`);
  if (usesDenoTest) {
    writer.writeLine(`const { pathToFileURL } = require("url");`);
    writer.writeLine(
      `const { testDefinitions } = require("${options.denoTestShimPackageName}");`,
    );
  }
  writer.blankLine();

  writer.writeLine("const filePaths = [");
  writer.indent(() => {
    for (const entryPoint of options.testEntryPoints) {
      writer.quote(toJsFilePath(entryPoint)).write(",").newLine();
    }
  });
  writer.writeLine("];").newLine();

  const isolateTestFiles = options.testIsolation === "process";
  writer.write("async function main()").block(() => {
    if (isolateTestFiles) {
      writeSpawnPerFile();
    }
    if (preloadPath != null) {
      if (options.includeScriptModule) {
        writer.writeLine(`process.chdir(__dirname + "/script");`);
        writer.write("try ").inlineBlock(() => {
          writer.write("require(").quote(`./script/${preloadPath}`).write(");")
            .newLine();
        }).write(" catch(err)").block(() => {
          writer.writeLine("console.error(err);");
          writer.writeLine("process.exit(1);");
        });
      }
      if (options.includeEsModule) {
        writer.writeLine(`process.chdir(__dirname + "/esm");`);
        writer.write("await import(").quote(`./esm/${preloadPath}`).write(");")
          .newLine();
      }
      writer.blankLine();
    }
    if (usesDenoTest) {
      writer.write("const testContext = ").inlineBlock(() => {
        writer.writeLine("process,");
        writer.writeLine("pc,");
      }).write(";").newLine();
    }
    if (isolateTestFiles) {
      writeTestFileRun();
    } else {
      writer.write("for (const [i, filePath] of filePaths.entries())")
        .block(() => {
          writer.write("if (i > 0)").block(() => {
            writer.writeLine(`console.log("");`);
          }).blankLine();

          writeTestFileRun();
        });
    }
  });
  writer.blankLine();

  if (options.denoTestShimPackageName != null) {
    writer.writeLine(`${getRunTestDefinitionsCode()}`);
    writer.blankLine();
  }

  writer.writeLine("main();");
  return writer.toString();

  function writeSpawnPerFile() {
    // run each test file in its own process so that the module state of a
    // test file doesn't leak into the next one, which is what `deno test`
    // does by running each file in its own isolate
    writer.writeLine("const fileIndexArg = process.argv[2];");
    writer.write("if (fileIndexArg == null)").block(() => {
      writer.writeLine(`const { spawnSync } = require("child_process");`);
      writer.writeLine("let failed = false;");
      writer.write("for (const i of filePaths.keys())").block(() => {
        writer.write("if (i > 0)").block(() => {
          writer.writeLine(`console.log("");`);
        });
        writer.writeLine(
          "const args = [...process.execArgv, __filename, String(i)];",
        );
        writer.writeLine(
          `const result = spawnSync(process.execPath, args, { stdio: "inherit" });`,
        );
        writer.write("if (result.error != null)").block(() => {
          writer.writeLine("console.error(result.error);");
        });
        writer.write("if (result.status !== 0)").block(() => {
          writer.writeLine("failed = true;");
        });
      });
      writer.write("if (failed)").block(() => {
        writer.writeLine("process.exitCode = 1;");
      });
      writer.writeLine("return;");
    }).blankLine();
    writer.writeLine("const filePath = filePaths[Number(fileIndexArg)];");
    writer.write("if (filePath == null)").block(() => {
      writer.writeLine(
        `console.error("Unknown test file index: " + fileIndexArg);`,
      );
      writer.writeLine("process.exitCode = 1;");
      writer.writeLine("return;");
    }).blankLine();
  }

  function writeTestFileRun() {
    if (options.includeScriptModule) {
      writer.writeLine(`const scriptPath = "./script/" + filePath;`);
      writer.writeLine(
        `console.log("Running tests in " + pc.underline(scriptPath) + "...\\n");`,
      );
      writer.writeLine(`process.chdir(__dirname + "/script");`);
      if (usesDenoTest) {
        writer.write(`const scriptTestContext = `).inlineBlock(() => {
          writer.writeLine("origin: pathToFileURL(filePath).toString(),");
          writer.writeLine("...testContext,");
        }).write(";").newLine();
      }
      writer.write("try ").inlineBlock(() => {
        writer.writeLine(`require(scriptPath);`);
      }).write(" catch(err)").block(() => {
        writer.writeLine("console.error(err);");
        writer.writeLine("process.exit(1);");
      });
      if (usesDenoTest) {
        writer.writeLine(
          "await runTestDefinitions(testDefinitions.splice(0, testDefinitions.length), scriptTestContext);",
        );
      }
    }

    if (options.includeEsModule) {
      if (options.includeScriptModule) {
        writer.blankLine();
      }
      writer.writeLine(`const esmPath = "./esm/" + filePath;`);
      writer.writeLine(
        `console.log("\\nRunning tests in " + pc.underline(esmPath) + "...\\n");`,
      );
      writer.writeLine(`process.chdir(__dirname + "/esm");`);
      if (usesDenoTest) {
        writer.write(`const esmTestContext = `).inlineBlock(() => {
          writer.writeLine("origin: pathToFileURL(filePath).toString(),");
          writer.writeLine("...testContext,");
        }).write(";").newLine();
      }
      writer.writeLine(`await import(esmPath);`);
      if (usesDenoTest) {
        writer.writeLine(
          "await runTestDefinitions(testDefinitions.splice(0, testDefinitions.length), esmTestContext);",
        );
      }
    }
  }
}

function getRunTestDefinitionsCode() {
  return runTestDefinitions.toString().replace(
    "export async function",
    "async function",
  );
}

function createWriter() {
  return new CodeBlockWriter({
    indentNumberOfSpaces: 2,
  });
}
