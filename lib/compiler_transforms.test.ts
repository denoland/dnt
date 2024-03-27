// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals } from "@std/assert";
import { ts } from "@ts-morph/bootstrap";
import { transformImportMeta } from "./compiler_transforms.ts";

function testImportReplacements(input: string, output: string, cjs = true) {
  const sourceFile = ts.createSourceFile(
    "file.ts",
    input,
    ts.ScriptTarget.Latest,
  );
  const newSourceFile = ts.transform(sourceFile, [transformImportMeta], {
    module: cjs ? ts.ModuleKind.CommonJS : ts.ModuleKind.ES2015,
  }).transformed[0];
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
}\n`,
  );
});

Deno.test("transform import.meta.main expressions in esModule", () => {
  testImportReplacements(
    "if (import.meta.main) { console.log('main'); }",
    `if ((import.meta.url === ("file:///" + process.argv[1].replace(/\\\\/g, "/")).replace(/\\/{3,}/, "///"))) {
    console.log("main");
}\n`,
    false,
  );
});

Deno.test("transform import.meta.resolve expressions", () => {
  testImportReplacements(
    "function test(specifier) { import.meta.resolve(specifier); }",
    `function test(specifier) { new URL(specifier, require("url").pathToFileURL(__filename).href).href; }\n`,
  );
});

Deno.test("transform import.meta.resolve expressions in esModule", () => {
  testImportReplacements(
    "function test(specifier) { import.meta.resolve(specifier); }",
    `function test(specifier) { new URL(specifier, import.meta.url).href; }\n`,
    false,
  );
});
