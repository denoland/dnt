// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { parse } from "jsr:@std/csv/parse"
import { assertEquals } from "jsr:@std/assert/assert_equals"

export function add(a: number, b: number) {
  const result = parse("a,b,c\n1,2,3\n4,5,6");
  assertEquals(result[0], ["a", "b", "c"]);
  return a + b;
}
