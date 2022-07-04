// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { add } from "./mod.ts";
import { assertEquals } from "https://deno.land/std@0.143.0/testing/asserts.ts";

Deno.test("should add in test project", () => {
  assertEquals(add(1, 2), 3);
});

Deno.test("should get properties on test context", async (t) => {
  const url = import.meta.url;
  if (t.origin !== url) {
    console.log(`Context origin: ${t.origin}`);
    console.log(`Import meta url: ${url}`);
    throw new Error("Origin was not correct.");
  }
  if (t.parent !== undefined) {
    throw new Error("Parent should have been undefined.");
  }
  await t.step("inner", (tInner) => {
    if (tInner.parent !== t) {
      throw new Error("The parent was not correct.");
    }
  });
});

Deno.test({
  name: "should ignore",
  ignore: true,
  fn() {
    throw new Error("did not ignore");
  },
});
