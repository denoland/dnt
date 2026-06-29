// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals } from "@std/assert";
import { shimOptionsToTransformShims } from "./shims.ts";
import type { PackageShim } from "../transform.ts";

Deno.test("should get when all true", () => {
  const result = shimOptionsToTransformShims({
    deno: true,
    prompts: true,
    blob: true,
    crypto: true,
    domException: true,
    undici: true,
    weakRef: true,
    webSocket: true,
    custom: [{
      package: {
        name: "main",
        version: "^1.2.3",
      },
      globalNames: ["main"],
    }],
    customDev: [{
      package: {
        name: "test",
        version: "^1.2.3",
      },
      globalNames: ["test"],
    }],
  });

  assertEquals(result.shims.length, 9);
  assertEquals(result.testShims.length, 10);
});

Deno.test("should get when all dev", () => {
  const result = shimOptionsToTransformShims({
    deno: "dev",
    prompts: "dev",
    blob: "dev",
    crypto: "dev",
    domException: "dev",
    undici: "dev",
    weakRef: "dev",
    webSocket: "dev",
  });

  assertEquals(result.shims.length, 0);
  assertEquals(result.testShims.length, 8);
});

Deno.test("should get when all false", () => {
  const result = shimOptionsToTransformShims({
    deno: false,
    prompts: false,
    blob: false,
    crypto: false,
    domException: false,
    undici: false,
    weakRef: false,
    webSocket: false,
  });

  assertEquals(result.shims.length, 0);
  assertEquals(result.testShims.length, 0);
});

Deno.test("should get when all undefined", () => {
  const result = shimOptionsToTransformShims({});

  assertEquals(result.shims.length, 0);
  assertEquals(result.testShims.length, 0);
});

Deno.test("should get for inner deno namespace", () => {
  const result = shimOptionsToTransformShims({
    deno: {
      test: true,
    },
  });

  assertEquals(result.shims.length, 1);
  assertEquals(
    (result.shims[0] as PackageShim).package.name,
    "@deno/shim-deno-test",
  );
  assertEquals(result.testShims.length, 1);
  assertEquals(
    (result.testShims[0] as PackageShim).package.name,
    "@deno/shim-deno-test",
  );
});
