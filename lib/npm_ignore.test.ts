// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals } from "@std/assert";
import { getNpmIgnoreText } from "./npm_ignore.ts";
import type { SourceMapOptions } from "./compiler.ts";

Deno.test("should include src directory when the source files are not necessary", () => {
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
  });
  runTest({
    sourceMaps: true,
    inlineSources: undefined,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
  });
  runTest({
    sourceMaps: "inline",
    inlineSources: undefined,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
  });

  runTest({
    sourceMaps: true,
    inlineSources: false,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
  });

  runTest({
    sourceMaps: undefined,
    inlineSources: true,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
  });
  runTest({
    sourceMaps: true,
    inlineSources: true,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
  });
  runTest({
    sourceMaps: "inline",
    inlineSources: true,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
  });
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: true,
    includeScriptModule: false,
    includeEsModule: true,
    declaration: "inline",
  });
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: false,
    declaration: "inline",
  });
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "separate",
  });
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: false,
  });
});

Deno.test("should include declaration maps of test files", () => {
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
    declarationMap: true,
  });
  runTest({
    sourceMaps: true,
    inlineSources: undefined,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
    declarationMap: true,
  });
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "separate",
    declarationMap: true,
  });
  // no declaration files, so no declaration maps
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: false,
    declarationMap: true,
  });
});

Deno.test("should keep the src directory when the declaration maps need it", () => {
  // declaration maps never inline their sources, so `inlineSources` does not
  // remove the need for the src directory like it does for source maps
  runTest({
    sourceMaps: true,
    inlineSources: true,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
    declarationMap: true,
  });
  runTest({
    sourceMaps: "inline",
    inlineSources: true,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "separate",
    declarationMap: true,
  });
  // nothing references the src directory, so it's excluded
  runTest({
    sourceMaps: true,
    inlineSources: true,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
    declaration: "inline",
    declarationMap: false,
  });
});

function runTest(options: {
  sourceMaps: SourceMapOptions | undefined;
  inlineSources: boolean | undefined;
  expectHasSrcFolder: boolean;
  declaration: "separate" | "inline" | false;
  declarationMap?: boolean;
  includeScriptModule: boolean | undefined;
  includeEsModule: boolean | undefined;
}) {
  const fileText = getNpmIgnoreText({
    sourceMap: options.sourceMaps,
    inlineSources: options.inlineSources,
    testFiles: [{
      filePath: "mod.test.ts",
      fileText: "",
    }],
    includeScriptModule: options.includeScriptModule,
    includeEsModule: options.includeEsModule,
    declaration: options.declaration,
    declarationMap: options.declarationMap,
  });

  assertEquals(fileText, getExpectedText());

  function getExpectedText() {
    let startText = options.expectHasSrcFolder ? "/src/\n" : "";
    if (!options.expectHasSrcFolder) {
      startText += "/src/mod.test.ts\n";
    }
    if (options.includeEsModule !== false) {
      startText += "/esm/mod.test.js\n";
      if (options.sourceMaps === true) {
        startText += "/esm/mod.test.js.map\n";
      }
      if (options.declaration === "inline") {
        startText += "/esm/mod.test.d.ts\n";
        if (options.declarationMap) {
          startText += "/esm/mod.test.d.ts.map\n";
        }
      }
    }
    if (options.includeScriptModule !== false) {
      startText += "/script/mod.test.js\n";
      if (options.sourceMaps === true) {
        startText += "/script/mod.test.js.map\n";
      }
      if (options.declaration === "inline") {
        startText += "/script/mod.test.d.ts\n";
        if (options.declarationMap) {
          startText += "/script/mod.test.d.ts.map\n";
        }
      }
    }
    if (options.declaration === "separate") {
      startText += "/types/mod.test.d.ts\n";
      if (options.declarationMap) {
        startText += "/types/mod.test.d.ts.map\n";
      }
    }

    return startText +
      `/test_runner.cjs
yarn.lock
pnpm-lock.yaml
`;
  }
}
