// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { parse } from "https://deno.land/std@0.109.0/flags/mod.ts";
import { ts } from "./mod.deps.ts";

export interface ParsedArgs {
  compilerOptions: ts.CompilerOptions;
  entryPoint: string;
  shimPackage: string | undefined;
  typeCheck: boolean;
}

export function parseArgs(
  args: string[],
): ParsedArgs | ts.Diagnostic[] | "help" {
  const cliArgs = parse(args, {
    alias: {
      h: "help",
    },
  });

  if (cliArgs.help || args.length === 0) {
    return "help";
  }

  const entryPoint = takeEntryPoint();
  const typeCheck = takeTypeCheck();
  const shimPackage = takeShimPackage();
  const tsArgs = ts.parseCommandLine(getRemainingArgs());

  if (tsArgs.errors.length > 0) {
    return tsArgs.errors;
  }

  return {
    compilerOptions: tsArgs.options,
    entryPoint,
    shimPackage,
    typeCheck,
  };

  function takeEntryPoint() {
    const firstArgument = cliArgs._.splice(0, 1)[0] as string;
    if (
      typeof firstArgument !== "string" || firstArgument.trim().length === 0
    ) {
      throw new Error(
        "Please specify an entry point (ex. `mod.ts`).",
      );
    }
    return firstArgument;
  }

  function takeTypeCheck() {
    const typeCheck = cliArgs.hasOwnProperty("typeCheck");
    delete cliArgs.typeCheck;
    return typeCheck;
  }

  function takeShimPackage() {
    const shimPackage = cliArgs.shimPackage;
    delete cliArgs.shimPackage;
    return shimPackage;
  }

  function getRemainingArgs() {
    const args = [];
    args.push(...cliArgs._);
    for (const [key, value] of Object.entries(cliArgs)) {
      if (key === "_") {
        continue;
      }
      args.push(`--${key}`);
      if (value != null) {
        args.push(value.toString());
      }
    }
    return args;
  }
}

export function outputUsage() {
  console.log(`Usage: dnt <entrypoint> [options]

Options:
  -h, --help              Shows the help message.
  --shimPackage <name>    Specifies the shim package name for 'Deno' namespace.
  --typeCheck             Performs type checking.

Compiler options:

  dnt supports the same compiler options that tsc supports.

    https://www.typescriptlang.org/docs/handbook/compiler-options.html

  For example, a small selection:

  --target <target>       Specifies the transpile target eg. ES6, ESNext, etc
  --outDir <dir>          The output directory (required)
  --declaration           Outputs the declaration files.

Examples:
  # Outputs to ./npm/dist
  dnt mod.ts --target ES6 --outDir ./npm/dist --declaration --module commonjs
`);
}
