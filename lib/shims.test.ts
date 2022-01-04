// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "./test.deps.ts";
import { shimOptionsToTransformShims } from "./shims.ts";

Deno.test("should get when all true", () => {
  const result = shimOptionsToTransformShims({
    deno: true,
    timers: true,
    prompts: true,
    blob: true,
    crypto: true,
    undici: true,
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

  assertEquals(result.shims.length, 7);
  assertEquals(result.testShims.length, 8);
});

Deno.test("should get when all dev", () => {
  const result = shimOptionsToTransformShims({
    deno: "dev",
    timers: "dev",
    prompts: "dev",
    blob: "dev",
    crypto: "dev",
    undici: "dev",
  });

  assertEquals(result.shims.length, 0);
  assertEquals(result.testShims.length, 6);
});

Deno.test("should get when all false", () => {
  const result = shimOptionsToTransformShims({
    deno: false,
    timers: false,
    prompts: false,
    blob: false,
    crypto: false,
    undici: false,
  });

  assertEquals(result.shims.length, 0);
  assertEquals(result.testShims.length, 0);
});

Deno.test("should get when all undefined", () => {
  const result = shimOptionsToTransformShims({});

  assertEquals(result.shims.length, 0);
  assertEquals(result.testShims.length, 0);
});
