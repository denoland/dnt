// Copyright 2018-2024 the Deno authors. MIT license.

import CodeBlockWriter from "code-block-writer";
import type { GlobalName, Shim } from "../../transform.ts";
import { isPathOrUrl } from "../utils.ts";
import { runTestDefinitions } from "./test_runner.ts";

/** The `Deno` namespace is never set as a global because code commonly checks
 * for its existence in order to detect the runtime it's running in.
 */
const nonGlobalShimNames = new Set(["Deno"]);

export function getTestRunnerCode(options: {
  testEntryPoints: string[];
  denoTestShimPackageName: string | undefined;
  includeEsModule: boolean | undefined;
  includeScriptModule: boolean | undefined;
  testShims?: Shim[];
}) {
  const usesDenoTest = options.denoTestShimPackageName != null;
  // the test code is transformed to use the shims, but the code being tested
  // is not, so set the globals in order for it to use them as well
  const globalShims = (options.testShims ?? [])
    .map((shim) => ({
      specifier: getShimSpecifier(shim),
      globalNames: shim.globalNames
        .map(toGlobalName)
        .filter((n) => !n.typeOnly && !nonGlobalShimNames.has(n.name)),
    }))
    .filter((shim) => shim.specifier != null && shim.globalNames.length > 0);
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
      writer.quote(entryPoint.replace(/\.ts$/, ".js")).write(",").newLine();
    }
  });
  writer.writeLine("];").newLine();

  writer.write("async function main()").block(() => {
    if (globalShims.length > 0) {
      writer.writeLine("await setUpGlobals();");
    }
    if (usesDenoTest) {
      writer.write("const testContext = ").inlineBlock(() => {
        writer.writeLine("process,");
        writer.writeLine("pc,");
      }).write(";").newLine();
    }
    writer.write("for (const [i, filePath] of filePaths.entries())")
      .block(() => {
        writer.write("if (i > 0)").block(() => {
          writer.writeLine(`console.log("");`);
        }).blankLine();

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
      });
  });
  writer.blankLine();

  if (globalShims.length > 0) {
    writer.write("async function setUpGlobals()").block(() => {
      for (const [i, shim] of globalShims.entries()) {
        const shimName = `shim${i}`;
        writer.writeLine(
          `const ${shimName} = await import(${
            JSON.stringify(shim.specifier)
          });`,
        );
        for (const globalName of shim.globalNames) {
          writer.writeLine(
            `defineGlobal(${JSON.stringify(globalName.name)}, ${shimName}.${
              globalName.exportName ?? globalName.name
            });`,
          );
        }
      }
      writer.blankLine();
      writer.write("function defineGlobal(name, value)").block(() => {
        writer.writeLine(
          "// some globals are defined as getters on globalThis (ex. `crypto`),",
        );
        writer.writeLine("// so a plain assignment would not work here");
        writer.write("Object.defineProperty(globalThis, name, ").inlineBlock(
          () => {
            writer.writeLine("value,");
            writer.writeLine("writable: true,");
            writer.writeLine("enumerable: false,");
            writer.writeLine("configurable: true,");
          },
        ).write(");").newLine();
      });
    });
    writer.blankLine();
  }

  if (options.denoTestShimPackageName != null) {
    writer.writeLine(`${getRunTestDefinitionsCode()}`);
    writer.blankLine();
  }

  writer.writeLine("main();");
  return writer.toString();
}

/** Gets the specifier to import the shim from in the test runner or
 * `undefined` when the shim can't be imported from there.
 */
function getShimSpecifier(shim: Shim) {
  if ("package" in shim) {
    return shim.package.subPath == null
      ? shim.package.name
      : `${shim.package.name}/${shim.package.subPath}`;
  }

  // local and remote modules are part of the graph, which the test runner
  // has no way of resolving to an output file
  return isPathOrUrl(shim.module) ? undefined : shim.module;
}

function toGlobalName(name: GlobalName | string): GlobalName {
  return typeof name === "string" ? { name } : name;
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
