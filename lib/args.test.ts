// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import {
  assertEquals,
  assertThrows,
} from "https://deno.land/std@0.109.0/testing/asserts.ts";
import { parseArgs, ParsedArgs } from "./args.ts";

Deno.test("help for no args", () => {
  assertEquals(parseArgs([]), "help");
});

Deno.test("help for -h and --help", () => {
  assertEquals(parseArgs(["-h"]), "help");
  assertEquals(parseArgs(["--help"]), "help");
});

Deno.test("error for non-string string arg", () => {
  assertThrows(
    () => parseArgs(["mod.ts", "--outDir"]),
    Error,
    "Expected string value for outDir.",
  );
});

Deno.test("error only shim package name", () => {
  assertThrows(
    () => parseArgs(["mod.ts", "--shimPackageName", "test"]),
    Error,
    "Both a shimPackageName and shimPackageVersion must be provided at the same time or not provided.",
  );
});

Deno.test("error only shim package version", () => {
  assertThrows(
    () => parseArgs(["mod.ts", "--shimPackageVersion", "0.1.0"]),
    Error,
    "Both a shimPackageName and shimPackageVersion must be provided at the same time or not provided.",
  );
});

Deno.test("get for just one argument", () => {
  const expectedArgs: ParsedArgs = {
    entryPoint: undefined,
    typeCheck: false,
    shimPackage: undefined,
    packageName: undefined,
    packageVersion: undefined,
    outDir: undefined,
    config: "dnt.json",
  };
  assertEquals(parseArgs(["--config", "dnt.json"]), expectedArgs);
});

Deno.test("get all args", () => {
  const expectedArgs: ParsedArgs = {
    entryPoint: "mod.ts",
    typeCheck: true,
    shimPackage: {
      name: "shim-package",
      version: "0.4.0",
    },
    packageName: "my-test-package",
    packageVersion: "1.0.0",
    config: "test.json",
    outDir: "dist",
  };
  assertEquals(
    parseArgs([
      "mod.ts",
      "--typeCheck",
      "--shimPackageName",
      "shim-package",
      "--shimPackageVersion",
      "0.4.0",
      "--packageName",
      "my-test-package",
      "--packageVersion",
      "1.0.0",
      "--outDir",
      "dist",
      "--config",
      "test.json",
    ]),
    expectedArgs,
  );
});

Deno.test("unknown argument", () => {
  assertThrows(
    () => parseArgs(["mod.ts", "--testing", "test"]),
    Error,
    "Unknown arguments: --testing test",
  );
});
