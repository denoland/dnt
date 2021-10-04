// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { parseArgs } from "./lib/_args.ts";
import { outputDiagnostics } from "./lib/_compiler.ts";
import { emit } from "./mod.ts";

const args = parseArgs(Deno.args);
if (args instanceof Array) {
  outputDiagnostics(args);
  Deno.exit(1);
}

const emitResult = await emit({
  compilerOptions: args.compilerOptions,
  entryPoint: args.entryPoint,
  shimPackageName: args.shimPackageName,
  typeCheck: args.typeCheck,
});

if (!emitResult.success) {
  outputDiagnostics(emitResult.diagnostics);
  Deno.exit(1);
}
