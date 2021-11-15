// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { add } from "./mod.ts";
import { assertEquals } from "https://deno.land/std@0.109.0/testing/asserts.ts";

Deno.test("should add in test project", () => {
  assertEquals(add(1, 2), 3);
});

function _testTypes(headers: Headers) {
  // this was previously erroring
  const _test: string | null = headers.get("some-header-name");
  const otherHeaders = new Headers();
  const _test2: string | null = otherHeaders.get("some-header-name");
}
