import { withResolvers } from "./mod.ts";

function assertEquals(a: unknown, b: unknown) {
  if (a !== b) {
    throw new Error(`${a} did not equal ${b}`);
  }
}

Deno.test("should resolve", async () => {
  const { promise, resolve } = withResolvers<number>();
  setTimeout(() => resolve(5), 10);
  assertEquals(await promise, 5);
});

Deno.test("should reject", async () => {
  const { promise, reject } = withResolvers<number>();
  setTimeout(() => reject(new Error("test")), 10);
  try {
    await promise;
    throw new Error("Did not throw.");
  } catch (err) {
    assertEquals((err as Error).message, "test");
  }
});
