// Copyright 2018-2024 the Deno authors. MIT license.

import { add } from "./mod.ts";
import { assertEquals } from "jsr:@std/assert@0.221/assert-equals";

Deno.test("should add in test project", () => {
  assertEquals(add(1, 2), 3);
});
