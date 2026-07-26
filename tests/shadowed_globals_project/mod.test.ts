// Copyright 2018-2024 the Deno authors. MIT license.

import text, { getText, Symbol } from "./mod.ts";

Deno.test("shadowed globals", () => {
  if (text !== "hello") {
    throw new Error("Failed.");
  }
  if (getText() !== "hello world") {
    throw new Error("Failed.");
  }
  if (new Symbol().text !== "hello world") {
    throw new Error("Failed.");
  }
});
