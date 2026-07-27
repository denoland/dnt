// Copyright 2018-2024 the Deno authors. MIT license.

import { describe, it } from "jsr:@std/testing@^1.0.19/bdd";
import { add } from "./mod.ts";

describe("a", () => {
  it("adds", () => {
    if (add(1, 1) !== 2) {
      throw new Error("Failed.");
    }
  });
});
