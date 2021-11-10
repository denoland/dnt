import {
  getCompilerScriptTarget,
  getCompilerSourceMapOptions,
  getTopLevelAwait,
  ScriptTarget,
  SourceMapOptions,
} from "./compiler.ts";
import { ts } from "./mod.deps.ts";
import { assertEquals, assertThrows } from "./test.deps.ts";

Deno.test("script target should have expected outputs", () => {
  const cases: {
    [k in ScriptTarget]: ts.ScriptTarget;
  } = {
    "ES3": ts.ScriptTarget.ES3,
    "ES5": ts.ScriptTarget.ES5,
    "ES2015": ts.ScriptTarget.ES2015,
    "ES2016": ts.ScriptTarget.ES2016,
    "ES2017": ts.ScriptTarget.ES2017,
    "ES2018": ts.ScriptTarget.ES2018,
    "ES2019": ts.ScriptTarget.ES2019,
    "ES2020": ts.ScriptTarget.ES2020,
    "ES2021": ts.ScriptTarget.ES2021,
    "Latest": ts.ScriptTarget.Latest,
  };

  for (const key in cases) {
    const scriptTarget = key as ScriptTarget;
    assertEquals(getCompilerScriptTarget(scriptTarget), cases[scriptTarget]);
  }

  assertEquals(getCompilerScriptTarget(undefined), ts.ScriptTarget.ES2021);
  assertThrows(() => getCompilerScriptTarget("invalid" as any));
});

Deno.test("get has top level await", () => {
  runTest("const some = code;class SomeOtherCode {}", undefined);
  runTest("async function test() { await 5; }", undefined);
  runTest("await test();", {
    line: 0,
    character: 0,
  });

  function runTest(code: string, expected: ts.LineAndCharacter | undefined) {
    const sourceFile = ts.createSourceFile(
      "file.ts",
      code,
      ts.ScriptTarget.Latest,
    );
    assertEquals(getTopLevelAwait(sourceFile), expected);
  }
});

Deno.test("get compiler options for source map option", () => {
  runTest("inline", { inlineSourceMap: true });
  runTest(true, { sourceMap: true });
  runTest(false, {});
  runTest(undefined, {});

  function runTest(
    useSourceMaps: SourceMapOptions | undefined,
    expected: { sourceMap?: boolean; inlineSourceMap?: boolean },
  ) {
    assertEquals(getCompilerSourceMapOptions(useSourceMaps), expected);
  }
});
