// Copyright 2018-2024 the Deno authors. MIT license.

import { assertEquals, assertThrows } from "@std/assert";
import {
  resolvePolyfillOptions,
  resolveUseImportMetaPolyfill,
} from "./polyfills.ts";

Deno.test("resolvePolyfillOptions - undefined leaves everything to the target", () => {
  assertEquals(resolvePolyfillOptions(undefined), {});
});

Deno.test("resolvePolyfillOptions - boolean applies to every polyfill", () => {
  assertEquals(resolvePolyfillOptions(false), {
    arrayFindLast: false,
    arrayFromAsync: false,
    errorCause: false,
    importMeta: false,
    objectHasOwn: false,
    promiseWithResolvers: false,
    stringReplaceAll: false,
  });
  assertEquals(
    Object.values(resolvePolyfillOptions(true)).every((v) => v),
    true,
  );
});

Deno.test("resolvePolyfillOptions - object only includes what's specified", () => {
  assertEquals(
    resolvePolyfillOptions({ importMeta: false, errorCause: true }),
    { importMeta: false, errorCause: true },
  );
});

Deno.test("resolvePolyfillOptions - undefined values fall back to the target", () => {
  assertEquals(
    resolvePolyfillOptions({ importMeta: undefined, errorCause: true }),
    { errorCause: true },
  );
});

Deno.test("resolvePolyfillOptions - throws for an unknown polyfill", () => {
  assertThrows(
    () =>
      resolvePolyfillOptions(
        { importMata: false } as Record<string, boolean>,
      ),
    Error,
    "Unknown polyfill 'importMata'",
  );
});

Deno.test("resolveUseImportMetaPolyfill - required when emitting a script module", () => {
  assertEquals(
    resolveUseImportMetaPolyfill({
      polyfills: {},
      target: "Latest",
      emitScriptModule: true,
    }),
    true,
  );
});

Deno.test("resolveUseImportMetaPolyfill - throws when disabled with a script module", () => {
  assertThrows(
    () =>
      resolveUseImportMetaPolyfill({
        polyfills: { importMeta: false },
        target: "ES2021",
        emitScriptModule: true,
      }),
    Error,
    "cannot be disabled when emitting a script module",
  );
});

Deno.test("resolveUseImportMetaPolyfill - esm only follows the target by default", () => {
  assertEquals(
    resolveUseImportMetaPolyfill({
      polyfills: {},
      target: "ES2021",
      emitScriptModule: false,
    }),
    true,
  );
  assertEquals(
    resolveUseImportMetaPolyfill({
      polyfills: {},
      target: "Latest",
      emitScriptModule: false,
    }),
    false,
  );
});

Deno.test("resolveUseImportMetaPolyfill - esm only respects an explicit override", () => {
  assertEquals(
    resolveUseImportMetaPolyfill({
      polyfills: { importMeta: false },
      target: "ES2021",
      emitScriptModule: false,
    }),
    false,
  );
  assertEquals(
    resolveUseImportMetaPolyfill({
      polyfills: { importMeta: true },
      target: "Latest",
      emitScriptModule: false,
    }),
    true,
  );
});
