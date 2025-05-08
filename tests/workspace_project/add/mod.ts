import { assertEquals } from "@std/assert";

export function add(a: number, b: number) {
  const value = a + b;
  assertEquals(value, a + b);
  return value;
}
