// Copyright 2018-2024 the Deno authors. MIT license.

import { add } from "./add.ts";

Deno.test("should add in cwd project", () => {
  if (add(1, 2) !== "marker: 3") {
    throw new Error("Did not add.");
  }
});
