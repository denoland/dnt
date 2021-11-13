// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { getResult } from "./mod.ts";

Deno.test("should get the result", () => {
  if (getResult() !== "test") {
    throw new Error("fail");
  }
});
