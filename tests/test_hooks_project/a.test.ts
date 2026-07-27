// Copyright 2018-2024 the Deno authors. MIT license.

import { beforeEach, describe, it } from "@std/testing/bdd";
import { add } from "./mod.ts";

let value = 0;

// a global hook can only be added once per test file, so this fails
// when the test files aren't run in their own process
beforeEach(() => {
  value = 1;
});

describe("a", () => {
  it("adds", () => {
    if (add(value, 1) !== 2) {
      throw new Error("Failed.");
    }
  });
});
