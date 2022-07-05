// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "./test.deps.ts";
import { getNpmIgnoreText } from "./npm_ignore.ts";
import { SourceMapOptions } from "./compiler.ts";

Deno.test("should include src directory when the source files are not necessary", () => {
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
  });
  runTest({
    sourceMaps: true,
    inlineSources: undefined,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
  });
  runTest({
    sourceMaps: "inline",
    inlineSources: undefined,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
  });

  runTest({
    sourceMaps: true,
    inlineSources: false,
    expectHasSrcFolder: false,
    includeScriptModule: true,
    includeEsModule: true,
  });

  runTest({
    sourceMaps: undefined,
    inlineSources: true,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
  });
  runTest({
    sourceMaps: true,
    inlineSources: true,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
  });
  runTest({
    sourceMaps: "inline",
    inlineSources: true,
    expectHasSrcFolder: true,
    includeScriptModule: true,
    includeEsModule: true,
  });
});

function runTest(options: {
  sourceMaps: SourceMapOptions | undefined;
  inlineSources: boolean | undefined;
  expectHasSrcFolder: boolean;
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
  });

  assertEquals(fileText, getExpectedText());

  function getExpectedText() {
    let startText = options.expectHasSrcFolder ? "src/\n" : "";
    startText += options.includeEsModule !== false ? "esm/mod.test.js\n" : "";
    startText += options.includeScriptModule !== false
      ? "script/mod.test.js\n"
      : "";
    return startText +
      `types/mod.test.d.ts
test_runner.js
yarn.lock
pnpm-lock.yaml
`;
  }
}
