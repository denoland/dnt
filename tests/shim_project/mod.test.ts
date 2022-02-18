// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
import { isDeno } from "https://deno.land/x/which_runtime@0.2.0/mod.ts";
import {
  addAsync,
  getCryptoKeyPair,
  localShimValue,
  throwDomException,
} from "./mod.ts";

Deno.test("should add in test project", async () => {
  const result = await addAsync(1, 2);
  if (result !== 3) {
    throw new Error(`Result fail: ${result}`);
  }
});

Deno.test("should get crypto key pair", async () => {
  const value = await getCryptoKeyPair([
    "encrypt" as const,
    "decrypt" as const,
  ]);
  if (value.privateKey == null || value.publicKey == null) {
    throw new Error("Was null.");
  }
});

Deno.test("should shim DOMException", () => {
  let err: DOMException | undefined = undefined;
  try {
    throwDomException();
  } catch (caughtErr) {
    err = caughtErr as DOMException;
  }
  if (!(err instanceof DOMException)) {
    throw new Error("Expected to throw DOMException.");
  }
});

Deno.test("should shim localValue", { ignore: isDeno }, () => {
  if (localShimValue().byteLength !== 100) {
    throw new Error("Did not equal expected.");
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
