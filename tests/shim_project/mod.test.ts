// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { addAsync } from "./mod.ts";

Deno.test("should add in test project", async () => {
  const result = await addAsync(1, 2);
  if (result !== 3) {
    throw new Error(`Result fail: ${result}`);
  }
});

function _testTypes(headers: Headers, blob: Blob) {
  // this was previously erroring
  const _test: string | null = headers.get("some-header-name");
  const createdHeaders = new Headers();
  const _test2: string | null = createdHeaders.get("some-header-name");

  const _testBlob1: number = blob.size;
  const createdBlob = new Blob([]);
  const _testBlob2: number = createdBlob.size;
}
