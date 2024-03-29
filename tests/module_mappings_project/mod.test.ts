// Copyright 2018-2024 the Deno authors. MIT license.

import { isDeno } from "https://deno.land/x/which_runtime@0.2.0/mod.ts";
import { output } from "./mod.ts";

Deno.test("should add in test project", () => {
  if (isDeno) {
    if (output() !== "deno") {
      throw new Error("Invalid output.");
    }
  } else {
    if (output() !== "node") {
      throw new Error("Invalid output.");
    }
  }
});
