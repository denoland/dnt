// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { parse } from "https://deno.land/std@0.109.0/flags/mod.ts";

export interface ParsedArgs {
  entryPoint: string | undefined;
  shimPackage: {
    name: string;
    version: string;
  } | undefined;
  typeCheck: boolean | undefined;
  packageName: string | undefined;
  packageVersion: string | undefined;
  config: string | undefined;
  outDir: string | undefined;
}

export function parseArgs(
  args: string[],
): ParsedArgs | "help" {
  const cliArgs = parse(args, {
    alias: {
      h: "help",
    },
  });

  if (cliArgs.help || args.length === 0) {
    return "help";
  }

  const result: ParsedArgs = {
    entryPoint: takeEntryPoint(),
    shimPackage: takeShimPackage(),
    typeCheck: takeBooleanProperty("typeCheck"),
    packageName: takeStringProperty("packageName"),
    packageVersion: takeStringProperty("packageVersion"),
    config: takeStringProperty("config"),
    outDir: takeStringProperty("outDir"),
  };

  const remainingArgs = getRemainingArgs();
  if (remainingArgs.length > 0) {
    throw new Error(`Unknown arguments: ${remainingArgs.join(" ")}`);
  }

  return result;

  function takeEntryPoint() {
    const firstArgument = cliArgs._.splice(0, 1)[0] as string | undefined;
    return firstArgument;
  }

  function takeShimPackage() {
    const shimPackageName = takeStringProperty("shimPackageName");
    const shimPackageVersion = takeStringProperty("shimPackageVersion");
    if (typeof shimPackageName !== typeof shimPackageVersion) {
      throw new Error(
        "Both a shimPackageName and shimPackageVersion must be provided at the same time or not provided.",
      );
    }
    if (!shimPackageName || !shimPackageVersion) {
      return undefined;
    }
    return {
      name: shimPackageName,
      version: shimPackageVersion,
    };
  }

  function takeBooleanProperty(name: string) {
    const hasProperty = cliArgs.hasOwnProperty(name);
    const value = cliArgs[name];
    delete cliArgs[name];
    if (value === false) {
      return false;
    }
    return hasProperty;
  }

  function takeStringProperty(name: string) {
    const value = cliArgs[name];
    delete cliArgs[name];
    if (value != null && typeof value !== "string") {
      throw new Error(`Expected string value for ${name}.`);
    }
    return value;
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
  -h, --help                  Shows the help message.
  --shimPackage <name>        Specifies the shim package name for 'Deno' namespace.
  --typeCheck                 Performs type checking.
  --outDir <dir>              Directory to output the files to.
  --packageName <name>        Name of the package.
  --packageVersion <version>  Version to use for the package.json
  --config <path>             Path to the config file to use for export.

Examples:
  # Outputs to ./npm/dist
  dnt mod.ts --outDir ./npm/dist --config dnt.config
`);
}
