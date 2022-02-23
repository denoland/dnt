// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "./test.deps.ts";
import { ts } from "./mod.deps.ts";
import { transformImportMeta } from "./compiler_transforms.ts";

function testImportReplacements(input: string, output: string) {
  const sourceFile = ts.createSourceFile(
    "file.ts",
    input,
    ts.ScriptTarget.Latest,
  );
  const newSourceFile =
    ts.transform(sourceFile, [transformImportMeta]).transformed[0];
  const text = ts.createPrinter({
    newLine: ts.NewLineKind.LineFeed,
  }).printFile(newSourceFile);

  assertEquals(text, output);
}

Deno.test("transform import.meta.url expressions", () => {
  testImportReplacements(
    "function test() { new URL(import.meta.url); }",
    `function test() { new URL(require("url").pathToFileURL(__filename).href); }\n`,
  );
});

Deno.test("transform import.meta.main expressions", () => {
  testImportReplacements(
    "if (import.meta.main) { console.log('main'); }",
    `if ((require.main === module)) {
    console.log("main");
}\n`);
});

