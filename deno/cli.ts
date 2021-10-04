// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { emit } from "./mod.ts";
import { parseArgs } from "./_args.ts";
import { outputDiagnostics } from "./_compiler.ts";

const args = parseArgs(Deno.args, diagnostics => {
  outputDiagnostics(diagnostics);
  Deno.exit(1);
});

await emit({
  compilerOptions: args.compilerOptions,
  entryPoint: args.entryPoint,
  shimPackageName: args.shimPackageName,
  typeCheck: args.typeCheck,
});
