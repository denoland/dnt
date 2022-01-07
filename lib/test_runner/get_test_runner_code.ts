// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { CodeBlockWriter } from "../mod.deps.ts";
import { runTestDefinitions } from "./test_runner.ts";

export function getTestRunnerCode(options: {
  testEntryPoints: string[];
  denoTestShimPackageName: string | undefined;
  includeCjs: boolean | undefined;
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

        if (options.includeCjs) {
          writer.writeLine(`const umdPath = "./umd/" + filePath;`);
          writer.writeLine(
            `console.log("Running tests in " + chalk.underline(umdPath) + "...\\n");`,
          );
          writer.writeLine(`process.chdir(__dirname + "/umd");`);
          writer.writeLine(`require(umdPath);`);
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
