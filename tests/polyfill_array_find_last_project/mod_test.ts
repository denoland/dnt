import { findLast } from "./mod.ts";

function assertEquals(a: unknown, b: unknown) {
  if (a !== b) {
    throw new Error(`${a} did not equal ${b}`);
  }
}

Deno.test("should find last in array", () => {
  assertEquals(findLast([1, 2, 3], () => false), undefined);
  assertEquals(findLast([1, 2, 3], () => true), 3);
  assertEquals(findLast([1, 2, 3], (value) => value === 1), 1);
  assertEquals(findLast([1, 2, 3], (_, index) => index === 1), 2);
  assertEquals(findLast([1, 2, 3], (value, _, obj) => obj[0] === value), 1);
  assertEquals(
    findLast([1, 2, 3], function (this: any) {
      if (this !== undefined) {
        throw new Error("Was not undefined.");
      }
      return true;
    }),
    3,
  );
  assertEquals(
    findLast([1, 2, 3], function (this: any[], value) {
      return this[0] === value;
    }, [2]),
    2,
  );
  assertEquals(
    findLast([
      { number: 2, other: 0 },
      { number: 2, other: 1 },
    ], (o) => o.number === 2)!.other,
    1,
  );
  assertEquals(
    findLast([
      { number: 2, other: 0 },
      { number: 2, other: 1 },
    ], (o) => o.number === 3),
    undefined,
  );
});

Deno.test("should find last for Uint8Array", () => {
  assertEquals(findLast(new Uint8Array([1, 2, 3]), () => true), 3);
});
