// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { parse } from "https://deno.land/std@0.109.0/flags/mod.ts";
import { parseArgs } from "./lib/_args.ts";
import { outputDiagnostics } from "./lib/_compiler.ts";
import { emit } from "./mod.ts";

const cliArgs = parse(Deno.args, {
  alias: {
    h: "help",
  },
});

if (cliArgs.help) {
  usage();
  Deno.exit(0);
}

const args = parseArgs(cliArgs);
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

function usage() {
  console.log(`Usage: dnt <entrypoint> [options]

Options:
  -h, --help              Show the help message.
  --target <target>       Specifies the transpile target eg. ES6, ESNext, etc
  --outDir <dir>          The output directory.
  --declaration           Outputs the declaration files.

Examples:
  # Outputs to ./npm/dist
  dnt mod.ts --target ES6 --outDir ./npm/dist --declaration
`);
}
