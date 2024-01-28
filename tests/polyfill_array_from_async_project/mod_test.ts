import { fromAsync } from "./mod.ts";

function assertEquals(a: unknown, b: unknown) {
  if (a !== b) {
    throw new Error(`${a} did not equal ${b}`);
  }
}

Deno.test("should get array from async generator", async () => {
  // example from https://www.npmjs.com/package/array-from-async
  async function *generator() {
    for (let i = 0; i < 4; i++)
      yield i * 2;
  }

  const result = await fromAsync(generator());
  assertEquals(result.length, 4);
  assertEquals(result[0], 0);
  assertEquals(result[1], 2);
  assertEquals(result[2], 4);
  assertEquals(result[3], 6);
});
