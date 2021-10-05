// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { Args } from "https://deno.land/std@0.109.0/flags/mod.ts";
import { ts } from "./_mod.deps.ts";

export interface ParsedArgs {
  compilerOptions: ts.CompilerOptions;
  entryPoint: string;
  shimPackageName: string | undefined;
  typeCheck: boolean;
}

export function parseArgs(cliArgs: Args): ParsedArgs | ts.Diagnostic[] {
  const entryPoint = takeEntryPoint();
  const typeCheck = takeTypeCheck();
  const shimPackageName = takeShimPackageName();
  const tsArgs = ts.parseCommandLine(getRemainingArgs());

  if (tsArgs.errors.length > 0) {
    return tsArgs.errors;
  }

  return {
    compilerOptions: tsArgs.options,
    entryPoint,
    shimPackageName,
    typeCheck,
  };

  function takeEntryPoint() {
    const firstArgument = cliArgs._.splice(0, 1)[0] as string;
    if (
      typeof firstArgument !== "string" || firstArgument.trim().length === 0
    ) {
      throw new Error(
        "Please specify an entry point as the first argument (ex. `mod.ts`).",
      );
    }
    return firstArgument;
  }

  function takeTypeCheck() {
    const typeCheck = cliArgs.hasOwnProperty("typeCheck");
    delete cliArgs.typeCheck;
    return typeCheck;
  }

  function takeShimPackageName() {
    const shimPackageName = cliArgs.shimPackageName;
    delete cliArgs.shimPackageName;
    return shimPackageName;
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
