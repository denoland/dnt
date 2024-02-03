// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "https://deno.land/std@0.181.0/testing/asserts.ts";
import { hasOwn, withResolvers } from "./mod.ts";

Deno.test("should test the polyfill", () => {
  assertEquals(hasOwn({}), false);
  assertEquals(hasOwn({ prop: 5 }), true);
});

Deno.test("with resolvers", async () => {
  const { promise, resolve } = withResolvers<number>();
  setTimeout(() => resolve(5), 10);
  const value = await promise;
  assertEquals(value, 5);
});
