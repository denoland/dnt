// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "./test.deps.ts";
import { ts } from "./mod.deps.ts";
import { transformImportMeta } from "./compiler_transforms.ts";

Deno.test("transform import.meta.url expressions", () => {
  const sourceFile = ts.createSourceFile(
    "file.ts",
    "function test() { new URL(import.meta.url); }",
    ts.ScriptTarget.Latest,
  );
  const newSourceFile =
    ts.transform(sourceFile, [transformImportMeta]).transformed[0];
  const text = ts.createPrinter({
    newLine: ts.NewLineKind.LineFeed,
  }).printFile(newSourceFile);

  assertEquals(
    text,
    `function test() { new URL(require("url").pathToFileURL(__filename).href); }\n`,
  );
});
