// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { add } from "./mod.ts";
import { assertEquals } from "https://deno.land/std@0.119.0/testing/asserts.ts";

Deno.test("should add in test project", () => {
  assertEquals(add(1, 2), 3);
});

Deno.test({
  name: "should ignore",
  ignore: true,
  fn() {
    throw new Error("did not ignore");
  },
});
