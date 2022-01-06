// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "./test.deps.ts";
import { getNpmIgnoreText } from "./npm_ignore.ts";
import { SourceMapOptions } from "./compiler.ts";

Deno.test("should include src directory when the source files are not necessary", () => {
  runTest({
    sourceMaps: undefined,
    inlineSources: undefined,
    expectHasSrcFolder: true,
  });
  runTest({
    sourceMaps: true,
    inlineSources: undefined,
    expectHasSrcFolder: false,
  });
  runTest({
    sourceMaps: "inline",
    inlineSources: undefined,
    expectHasSrcFolder: false,
  });

  runTest({
    sourceMaps: true,
    inlineSources: false,
    expectHasSrcFolder: false,
  });

  runTest({
    sourceMaps: undefined,
    inlineSources: true,
    expectHasSrcFolder: true,
  });
  runTest({
    sourceMaps: true,
    inlineSources: true,
    expectHasSrcFolder: true,
  });
  runTest({
    sourceMaps: "inline",
    inlineSources: true,
    expectHasSrcFolder: true,
  });
});

function runTest(options: {
  sourceMaps: SourceMapOptions | undefined;
  inlineSources: boolean | undefined;
  expectHasSrcFolder: boolean;
}) {
  const fileText = getNpmIgnoreText({
    sourceMap: options.sourceMaps,
    inlineSources: options.inlineSources,
    testFiles: [{
      filePath: "mod.test.ts",
      fileText: "",
    }],
  });

  assertEquals(fileText, getExpectedText());

  function getExpectedText() {
    const startText = options.expectHasSrcFolder ? "src/\n" : "";
    return startText + `esm/mod.test.js
umd/mod.test.js
test_runner.js
yarn.lock
pnpm-lock.yaml
`;
  }
}
