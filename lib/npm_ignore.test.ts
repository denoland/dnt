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

function runTest(options: {
  sourceMaps: SourceMapOptions | undefined;
  inlineSources: boolean | undefined;
  expectHasSrcFolder: boolean;
  declaration: "separate" | "inline" | false;
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
  });

  assertEquals(fileText, getExpectedText());

  function getExpectedText() {
    let startText = options.expectHasSrcFolder ? "/src/\n" : "";
    if (options.includeEsModule !== false) {
      startText += "/esm/mod.test.js\n";
      if (options.sourceMaps === true) {
        startText += "/esm/mod.test.js.map\n";
      }
      if (options.declaration === "inline") {
        startText += "/esm/mod.test.d.ts\n";
      }
    }
    if (options.includeScriptModule !== false) {
      startText += "/script/mod.test.js\n";
      if (options.sourceMaps === true) {
        startText += "/script/mod.test.js.map\n";
      }
      if (options.declaration === "inline") {
        startText += "/script/mod.test.d.ts\n";
      }
    }
    if (options.declaration === "separate") {
      startText += "/types/mod.test.d.ts\n";
    }

    return startText +
      `/test_runner.js
yarn.lock
pnpm-lock.yaml
`;
  }
}
