// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { add } from "./mod.ts";

Deno.test("should add in test project", () => {
  if (add(1, 2) !== 3) {
    throw new Error("FAIL");
  }
});
