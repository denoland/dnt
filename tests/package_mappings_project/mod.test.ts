// Copyright 2018-2024 the Deno authors. MIT license.

import { getResult } from "./mod.ts";

Deno.test("should get the result", () => {
  if (getResult() !== "test") {
    throw new Error("fail");
  }
});
