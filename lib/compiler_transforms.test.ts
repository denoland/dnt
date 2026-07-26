// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals } from "@std/assert";
import { createProjectSync, ts } from "@ts-morph/bootstrap";
import {
  createShadowedGlobalsTransformer,
  transformImportMeta,
} from "./compiler_transforms.ts";

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

Deno.test("does not transform new.target in commonjs", () => {
  testImportReplacementsCjs(
    "function test(...args) { return Reflect.construct(Struct, args, new.target); }",
    `function test(...args) { return Reflect.construct(Struct, args, new.target); }\n`,
  );
});

Deno.test("does not transform new.target in esModule", () => {
  testImportReplacementsEsm(
    "function test(...args) { return Reflect.construct(Struct, args, new.target); }",
    `function test(...args) { return Reflect.construct(Struct, args, new.target); }\n`,
  );
});

// the emit for every commonjs module starts with this
const cjsPrologue =
  `"use strict";\nObject.defineProperty(exports, "__esModule", { value: true });\n`;

function testShadowedGlobals(
  input: string,
  output: string,
  module = ts.ModuleKind.CommonJS,
) {
  const project = createProjectSync({
    useInMemoryFileSystem: true,
    compilerOptions: {
      module,
      target: ts.ScriptTarget.ES2022,
      esModuleInterop: true,
    },
  });
  project.createSourceFile("/mod.ts", input);
  const program = project.createProgram();
  let text: string | undefined;
  program.emit(
    undefined,
    (_filePath, data) => text = data,
    undefined,
    false,
    { before: [createShadowedGlobalsTransformer(program)] },
  );
  const prologue = module === ts.ModuleKind.CommonJS ? cjsPrologue : "";
  assertEquals(text, prologue + output);
}

const testShadowedGlobalsEsm = (input: string, output: string) =>
  testShadowedGlobals(input, output, ts.ModuleKind.ES2015);

Deno.test("renames declarations shadowing the globals the emit uses", () => {
  testShadowedGlobals(
    `const Object = "hello";\nlet [require] = ["world"];\nexport default Object + require;\n`,
    `const Object_1 = "hello";\nlet [require_1] = ["world"];\nexports.default = Object_1 + require_1;\n`,
  );
});

Deno.test("uses an unused name when renaming", () => {
  testShadowedGlobals(
    `const Object_1 = 1;\nconst Object = 2;\nexport default Object + Object_1;\n`,
    `const Object_1 = 1;\nconst Object_2 = 2;\nexports.default = Object_2 + Object_1;\n`,
  );
});

Deno.test("keeps the export names of renamed declarations", () => {
  testShadowedGlobals(
    `export class Object {}\nfunction Symbol() {}\nexport { Symbol as other, Symbol as default };\n`,
    `exports.other = exports.Object = void 0;\nclass Object_1 {\n}\nfunction Symbol_1() { }\nexports.Object = Object_1;\nexports.other = Symbol_1;\nexports.default = Symbol_1;\n`,
  );
});

Deno.test("renames shorthand properties and object binding patterns", () => {
  testShadowedGlobals(
    `const Object = { Object: 1 };\nconst { Object: value } = Object;\nexport default { Object, value };\n`,
    `const Object_1 = { Object: 1 };\nconst { Object: value } = Object_1;\nexports.default = { Object: Object_1, value };\n`,
  );
});

Deno.test("does not rename declarations in other scopes", () => {
  testShadowedGlobals(
    `export const keys = Object.keys({});\nexport function test() {\n  const Object = 1;\n  return Object;\n}\n`,
    `exports.keys = void 0;\nexports.test = test;\nexports.keys = Object.keys({});\nfunction test() {\n    const Object = 1;\n    return Object;\n}\n`,
  );
});

Deno.test("does not rename exported variables, which don't create a binding", () => {
  testShadowedGlobals(
    `export const Object = "hello";\nexport const length = Object.length;\n`,
    `exports.length = exports.Object = void 0;\nexports.Object = "hello";\nexports.length = exports.Object.length;\n`,
  );
});

Deno.test("renames declarations in an es module", () => {
  // the es module emit uses these globals when downleveling
  // (ex. `Object.defineProperty(this, "prop", ...)` for class fields)
  testShadowedGlobalsEsm(
    `const Object = "hello";\nexport default Object;\n`,
    `const Object_1 = "hello";\nexport default Object_1;\n`,
  );
});

Deno.test("keeps the export names in an es module", () => {
  testShadowedGlobalsEsm(
    `export const Object = 1;\nexport class Symbol {}\nexport { Object as other };\n`,
    `const Object_1 = 1;\nclass Symbol_1 {\n}\nexport { Object_1 as other };\nexport { Object_1 as Object, Symbol_1 as Symbol };\n`,
  );
});
