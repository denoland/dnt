import { getCompilerScriptTarget, ScriptTarget } from "./compiler.ts";
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
