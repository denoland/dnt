// Copyright 2018-2024 the Deno authors. MIT license.

import { getExistingGlobalNames } from "./mod.ts";
import { assertEquals } from "https://deno.land/std@0.181.0/testing/asserts.ts";

Deno.test("should set the globals of the built-in shims", () => {
  // node does not have these globals, so they can only exist
  // when the test runner sets them from `@deno/shim-prompts`
  assertEquals(
    getExistingGlobalNames(["alert", "confirm", "prompt"]),
    ["alert", "confirm", "prompt"],
  );
});

Deno.test("should set the globals of the custom shims", () => {
  assertEquals(getExistingGlobalNames(["Blob"]), ["Blob"]);
});
