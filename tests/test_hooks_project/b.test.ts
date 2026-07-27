// Copyright 2018-2024 the Deno authors. MIT license.

import { beforeEach, describe, it } from "jsr:@std/testing@^1.0.19/bdd";
import { add } from "./mod.ts";

let value = 0;

// a global hook errors when another test file already registered a global
// test, so this only works when each test file runs in its own process
beforeEach(() => {
  value = 1;
});

describe("b", () => {
  it("adds", () => {
    if (add(value, 1) !== 2) {
      throw new Error("Failed.");
    }
  });
});
