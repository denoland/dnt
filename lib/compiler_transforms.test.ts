// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals } from "@std/assert";
import { ts } from "@ts-morph/bootstrap";
import { transformImportMeta } from "./compiler_transforms.ts";

function testImportReplacements(
  input: string,
  output: string,
  module: ts.ModuleKind,
) {
  const sourceFile = ts.createSourceFile(
    "file.ts",
    input,
    ts.ScriptTarget.Latest,
  );
  const newSourceFile = ts.transform(sourceFile, [transformImportMeta], {
    module,
  }).transformed[0];
  const text = ts.createPrinter({
    newLine: ts.NewLineKind.LineFeed,
  }).printFile(newSourceFile);

  assertEquals(text, output);
}
const testImportReplacementsEsm = (input: string, output: string) =>
  testImportReplacements(input, output, ts.ModuleKind.ES2015);
const testImportReplacementsCjs = (input: string, output: string) =>
  testImportReplacements(input, output, ts.ModuleKind.CommonJS);

Deno.test("transform import.meta.url expressions in commonjs", () => {
  testImportReplacementsCjs(
    "function test() { new URL(import.meta.url); }",
    `function test() { new URL(globalThis[Symbol.for("import-meta-ponyfill-commonjs")](require, module).url); }\n`,
  );
});
Deno.test("transform import.meta.url expressions in esModule", () => {
  testImportReplacementsEsm(
    "function test() { new URL(import.meta.url); }",
    `function test() { new URL(globalThis[Symbol.for("import-meta-ponyfill-esmodule")](import.meta).url); }\n`,
  );
});

Deno.test("transform import.meta.main expressions in commonjs", () => {
  testImportReplacementsCjs(
    "if (import.meta.main) { console.log('main'); }",
    `if (globalThis[Symbol.for("import-meta-ponyfill-commonjs")](require, module).main) {
    console.log("main");
}\n`,
  );
});

Deno.test("transform import.meta.main expressions in esModule", () => {
  testImportReplacementsEsm(
    "export const isMain = import.meta.main;",
    `export const isMain = globalThis[Symbol.for("import-meta-ponyfill-esmodule")](import.meta).main;\n`,
  );
});

Deno.test("transform import.meta.resolve expressions", () => {
  testImportReplacementsCjs(
    "function test(specifier) { import.meta.resolve(specifier); }",
    `function test(specifier) { globalThis[Symbol.for("import-meta-ponyfill-commonjs")](require, module).resolve(specifier); }\n`,
  );
});

Deno.test("transform import.meta.resolve expressions in esModule", () => {
  testImportReplacementsEsm(
    "function test(specifier) { import.meta.resolve(specifier); }",
    `function test(specifier) { globalThis[Symbol.for("import-meta-ponyfill-esmodule")](import.meta).resolve(specifier); }\n`,
  );
});
