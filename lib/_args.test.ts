// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { assertEquals, assertThrows } from "https://deno.land/std@0.109.0/testing/asserts.ts";
import { parseArgs } from "./_args.ts";
import { ts } from "./_mod.deps.ts";

Deno.test("help for no args", () => {
  assertEquals(parseArgs([]), "help");
});

Deno.test("help for -h and --help", () => {
  assertEquals(parseArgs(["-h"]), "help");
  assertEquals(parseArgs(["--help"]), "help");
});

Deno.test("error for no entry point", () => {
  assertThrows(() => parseArgs(["--outDir", "test"]), Error, "Please specify an entry point (ex. `mod.ts`)");
});

Deno.test("get minimal amount of args", () => {
  assertEquals(parseArgs(["mod.ts"]), {
    entryPoint: "mod.ts",
    typeCheck: false,
    shimPackage: undefined,
    compilerOptions: {},
  });
});

Deno.test("get all args and compiler options", () => {
  assertEquals(parseArgs(["mod.ts", "--typeCheck", "--shimPackage", "shim-package", "--outDir", "dist"]), {
    entryPoint: "mod.ts",
    typeCheck: true,
    shimPackage: "shim-package",
    compilerOptions: {
      outDir: "dist"
    },
  });
});

Deno.test("diagnostic from ts compiler for unknown argument", () => {
  const diagnostics = parseArgs(["mod.ts", "--testing", "test"]) as ts.Diagnostic[];
  assertEquals(diagnostics.length, 1);
  // not the best, but it gets the point across (message from ts compiler)
  assertEquals(diagnostics[0].messageText, "Unknown compiler option '--testing'.");
});

Deno.test("diagnostic from ts compiler for invalid argument", () => {
  const diagnostics = parseArgs(["mod.ts", "--target", "test"]) as ts.Diagnostic[];
  assertEquals(diagnostics.length, 1);
  assertEquals(
    diagnostics[0].messageText,
    "Argument for '--target' option must be: 'es3', 'es5', 'es6', 'es2015', 'es2016', 'es2017', 'es2018', 'es2019', 'es2020', 'es2021', 'esnext'.",
  );
});
