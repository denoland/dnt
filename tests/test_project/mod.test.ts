// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { add } from "./mod.ts";
import { assertEquals } from "https://deno.land/std@0.109.0/testing/asserts.ts";

Deno.test("should add in test project", () => {
  assertEquals(add(1, 2), 3);
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
