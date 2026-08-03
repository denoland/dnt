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

Deno.test("should ignore a dependency's declaration file", () => {
  // a dependency that ships a declaration file (ex. a prebuilt package
  // published to JSR) is copied as-is rather than compiled to `.js` + `.d.ts`
  const depDts = "deps/jsr.io/@scope/dep/1.0.0/dep.d.ts";

  function run(options: {
    declaration: "separate" | "inline" | false;
    declarationMap?: boolean;
    includeEsModule?: boolean;
    includeScriptModule?: boolean;
  }) {
    return getNpmIgnoreText({
      sourceMap: undefined,
      inlineSources: undefined,
      testFiles: [{ filePath: depDts, fileText: "" }],
      includeEsModule: options.includeEsModule ?? true,
      includeScriptModule: options.includeScriptModule ?? true,
      declaration: options.declaration,
      declarationMap: options.declarationMap,
    });
  }

  // inline declarations are copied beside the emitted code
  assertEquals(
    run({ declaration: "inline" }),
    `/src/\n/esm/${depDts}\n/script/${depDts}\n` +
      "/test_runner.cjs\nyarn.lock\npnpm-lock.yaml\n",
  );
  // inline declarations with declaration maps (the maps reference `/src/`, so
  // the individual source files are ignored instead of the whole directory)
  assertEquals(
    run({ declaration: "inline", declarationMap: true }),
    `/src/${depDts}\n/esm/${depDts}\n/esm/${depDts}.map\n/script/${depDts}\n` +
      `/script/${depDts}.map\n/test_runner.cjs\nyarn.lock\npnpm-lock.yaml\n`,
  );
  // separate declarations only end up in the types directory
  assertEquals(
    run({ declaration: "separate" }),
    `/src/\n/types/${depDts}\n/test_runner.cjs\nyarn.lock\npnpm-lock.yaml\n`,
  );
  // no declarations means the file is never emitted
  assertEquals(
    run({ declaration: false }),
    `/src/\n/test_runner.cjs\nyarn.lock\npnpm-lock.yaml\n`,
  );
  // only the esm output is emitted
  assertEquals(
    run({ declaration: "inline", includeScriptModule: false }),
    `/src/\n/esm/${depDts}\n/test_runner.cjs\nyarn.lock\npnpm-lock.yaml\n`,
  );
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
