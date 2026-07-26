// Copyright 2018-2024 the Deno authors. MIT license.

import { getExistingGlobalNames } from "./mod.ts";
import { assertEquals } from "https://deno.land/std@0.181.0/testing/asserts.ts";

Deno.test("should have the globals set by the preload module", () => {
  assertEquals(
    getExistingGlobalNames(["Blob", "alert"]),
    ["Blob", "alert"],
  );
});
