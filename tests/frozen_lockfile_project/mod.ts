// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals } from "jsr:@std/assert@0.221/assert-equals";

export function add(a: number, b: number) {
  assertEquals(typeof a, "number");
  return a + b;
}
