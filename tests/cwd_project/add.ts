// Copyright 2018-2024 the Deno authors. MIT license.

import { marker } from "@dnt/marker";

export function add(a: number, b: number) {
  return `${marker}: ${a + b}`;
}
