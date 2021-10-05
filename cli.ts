// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { outputUsage, parseArgs } from "./lib/args.ts";
import { outputDiagnostics } from "./lib/compiler.ts";
import { emit } from "./mod.ts";

const args = parseArgs(Deno.args);

if (args === "help") {
  outputUsage();
  Deno.exit(0);
} else if (args instanceof Array) {
  outputDiagnostics(args);
  Deno.exit(1);
}

const emitResult = await emit({
  compilerOptions: args.compilerOptions,
  entryPoint: args.entryPoint,
  shimPackageName: args.shimPackage,
  typeCheck: args.typeCheck,
});

if (!emitResult.success) {
  outputDiagnostics(emitResult.diagnostics);
  Deno.exit(1);
}
