// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { assertEquals, assertThrows } from "https://deno.land/std@0.109.0/testing/asserts.ts";
import { parseArgs } from "./args.ts";
import { DiagnosticsError } from "./compiler.ts";
import { ts } from "./mod.deps.ts";

Deno.test("help for no args", () => {
  assertEquals(parseArgs([]), "help");
});

Deno.test("help for -h and --help", () => {
  assertEquals(parseArgs(["-h"]), "help");
  assertEquals(parseArgs(["--help"]), "help");
});

Deno.test("error for non-string string arg", () => {
  assertThrows(() => parseArgs(["mod.ts", "--shimPackage"]), Error, "Expected string value for shimPackage.");
});

Deno.test("get minimal amount of args", () => {
  assertEquals(parseArgs(["mod.ts"]), {
    entryPoint: "mod.ts",
    typeCheck: false,
    shimPackage: undefined,
    packageVersion: undefined,
    config: undefined,
    compilerOptions: {},
  });
});

Deno.test("get for just config", () => {
  assertEquals(parseArgs(["--config", "dnt.json"]), {
    entryPoint: undefined,
    typeCheck: false,
    shimPackage: undefined,
    packageVersion: undefined,
    config: "dnt.json",
    compilerOptions: {},
  });
});

Deno.test("get all args and compiler options", () => {
  assertEquals(parseArgs(["mod.ts", "--typeCheck", "--shimPackage", "shim-package", "--packageVersion", "1.0.0", "--outDir", "dist"]), {
    entryPoint: "mod.ts",
    typeCheck: true,
    shimPackage: "shim-package",
    packageVersion: "1.0.0",
    config: undefined,
    compilerOptions: {
      outDir: "dist"
    },
  });
});

Deno.test("diagnostic from ts compiler for unknown argument", () => {
  let diagnostics!: readonly ts.Diagnostic[];
  try {
    parseArgs(["mod.ts", "--testing", "test"]);
  } catch (err) {
    diagnostics = (err as DiagnosticsError).diagnostics;
  }
  assertEquals(diagnostics.length, 1);
  // not the best, but it gets the point across (message from ts compiler)
  assertEquals(diagnostics[0].messageText, "Unknown compiler option '--testing'.");
});

Deno.test("diagnostic from ts compiler for invalid argument", () => {
  let diagnostics!: readonly ts.Diagnostic[];
  try {
    parseArgs(["mod.ts", "--target", "test"]);
  } catch (err) {
    diagnostics = (err as DiagnosticsError).diagnostics;
  }
  assertEquals(diagnostics.length, 1);
  assertEquals(
    diagnostics[0].messageText,
    "Argument for '--target' option must be: 'es3', 'es5', 'es6', 'es2015', 'es2016', 'es2017', 'es2018', 'es2019', 'es2020', 'es2021', 'esnext'.",
  );
});
