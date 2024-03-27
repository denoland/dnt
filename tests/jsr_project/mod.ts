// Copyright 2018-2024 the Deno authors. MIT license.

import { parse } from "jsr:@std/csv/parse";
import { assertEquals } from "jsr:@std/assert/assert_equals";
import * as fs from "node:fs";

export function add(a: number, b: number) {
  console.log(fs.readFileSync);
  const result = parse("a,b,c\n1,2,3\n4,5,6");
  assertEquals(result[0], ["a", "b", "c"]);
  return a + b;
}
