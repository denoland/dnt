// Copyright 2018-2024 the Deno authors. MIT license.

import { add } from "./mod.ts";

Deno.test("should add in test project", () => {
  if (add(1, 2) !== 3) {
    throw new Error("FAIL");
  }
});
