// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { CodeBlockWriter } from "../mod.deps.ts";
import { runTestDefinitions } from "./test_runner.ts";

export function getTestRunnerCode(options: {
  testEntryPoints: string[];
  denoTestShimPackageName: string | undefined;
  includeScriptModule: boolean | undefined;
}) {
  const writer = createWriter();
  writer.writeLine(`const chalk = require("chalk");`)
    .writeLine(`const process = require("process");`);
  if (options.denoTestShimPackageName != null) {
    writer.writeLine(
      `const { testDefinitions } = require("${options.denoTestShimPackageName}");`,
    );
  }
  writer.blankLine();

  writer.writeLine("const filePaths = [");
  writer.indent(() => {
    for (const entryPoint of options.testEntryPoints) {
      writer.quote(entryPoint.replace(/\.ts$/, ".js")).write(",").newLine();
    }
  });
  writer.writeLine("];").newLine();

  writer.write("async function main()").block(() => {
    writer.write("const testContext = ").inlineBlock(() => {
      writer.writeLine("process,");
      writer.writeLine("chalk,");
    }).write(";").newLine();
    writer.write("for (const [i, filePath] of filePaths.entries())")
      .block(() => {
        writer.write("if (i > 0)").block(() => {
          writer.writeLine(`console.log("");`);
        }).blankLine();

        if (options.includeScriptModule) {
          writer.writeLine(`const scriptPath = "./script/" + filePath;`);
          writer.writeLine(
            `console.log("Running tests in " + chalk.underline(scriptPath) + "...\\n");`,
          );
          writer.writeLine(`process.chdir(__dirname + "/script");`);
          writer.write("try ").inlineBlock(() => {
            writer.writeLine(`require(scriptPath);`);
          }).write(" catch(err)").block(() => {
            writer.writeLine("console.error(err);");
            writer.writeLine("process.exit(1);");
          });
          if (options.denoTestShimPackageName != null) {
            writer.writeLine(
              "await runTestDefinitions(testDefinitions.splice(0, testDefinitions.length), testContext);",
            );
          }
          writer.blankLine();
        }

        writer.writeLine(`const esmPath = "./esm/" + filePath;`);
        writer.writeLine(`process.chdir(__dirname + "/esm");`);
        writer.writeLine(
          `console.log("\\nRunning tests in " + chalk.underline(esmPath) + "...\\n");`,
        );
        writer.writeLine(`await import(esmPath);`);
        if (options.denoTestShimPackageName != null) {
          writer.writeLine(
            "await runTestDefinitions(testDefinitions.splice(0, testDefinitions.length), testContext);",
          );
        }
      });
  });
  writer.blankLine();

  if (options.denoTestShimPackageName != null) {
    writer.writeLine(`${getRunTestDefinitionsCode()}`);
    writer.blankLine();
  }

  writer.writeLine("main();");
  return writer.toString();
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
