// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { parse } from "https://deno.land/std@0.109.0/flags/mod.ts";
import { ts } from "./_mod.deps.ts";

export interface ParsedArgs {
  compilerOptions: ts.CompilerOptions;
  entryPoint: string;
  shimPackageName: string | undefined;
  typeCheck: boolean,
}

export function parseArgs(
  cliArgs: string[],
  reportDiagnostics: (diagnostics: ts.Diagnostic[]) => void,
): ParsedArgs {
  const parsedArgs = parse(cliArgs);
  const entryPoint = takeEntryPoint();
  const typeCheck = takeTypeCheck();
  const shimPackageName = takeShimPackageName();
  const tsArgs = ts.parseCommandLine(getRemainingArgs());

  if (tsArgs.errors.length > 0) {
    reportDiagnostics(tsArgs.errors);
  }

  return {
    compilerOptions: tsArgs.options,
    entryPoint,
    shimPackageName,
    typeCheck,
  };

  function takeEntryPoint() {
    const firstArgument = parsedArgs._.splice(0, 1)[0] as string;
    if (typeof firstArgument !== "string" || firstArgument.trim().length === 0) {
      throw new Error("Please specify an entry point as the first argument (ex. `mod.ts`).");
    }
    return firstArgument;
  }

  function takeTypeCheck() {
    const typeCheck = parsedArgs.hasOwnProperty("typeCheck");
    delete parsedArgs.typeCheck;
    return typeCheck;
  }

  function takeShimPackageName() {
    const shimPackageName = parsedArgs.shimPackageName;
    delete parsedArgs.shimPackageName;
    return shimPackageName;
  }

  function getRemainingArgs() {
    const args = [];
    args.push(...parsedArgs._);
    for (const [key, value] of Object.entries(parsedArgs)) {
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
