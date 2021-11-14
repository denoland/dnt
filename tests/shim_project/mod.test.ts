// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { addAsync } from "./mod.ts";

Deno.test("should add in test project", async () => {
  const result = await addAsync(1, 2);
  if (result !== 3) {
    throw new Error(`Result fail: ${result}`);
  }
});
