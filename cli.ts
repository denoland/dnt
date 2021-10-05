// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { outputUsage, parseArgs, ParsedArgs } from "./lib/args.ts";
import { path } from "./lib/mod.deps.ts";
import { DiagnosticsError, outputDiagnostics } from "./lib/compiler.ts";
import { resolveArgs } from "./lib/resolve_args.ts";
import { emit } from "./mod.ts";

try {
  const cliArgs = parseArgs(Deno.args);

  if (cliArgs === "help") {
    outputUsage();
    Deno.exit(0);
  }

  const resolvedArgs = resolveArgs(cliArgs);

  const emitResult = await emit({
    compilerOptions: resolvedArgs.compilerOptions,
    entryPoint: resolvedArgs.entryPoint,
    shimPackageName: resolvedArgs.shimPackage,
    typeCheck: resolvedArgs.typeCheck,
  });

  if (resolvedArgs.package) {
    Deno.writeTextFileSync(
      path.join(resolvedArgs.compilerOptions.outDir!, "package.json"),
      JSON.stringify(resolvedArgs.package, undefined, 2),
    );
  }

  if (!emitResult.success) {
    outputDiagnostics(emitResult.diagnostics);
    Deno.exit(1);
  }
} catch (err) {
  if (err instanceof DiagnosticsError) {
    outputDiagnostics(err.diagnostics);
    Deno.exit(1);
  }
  throw err;
}
