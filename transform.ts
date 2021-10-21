// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { path } from "./lib/transform.deps.ts";
import init, * as wasmFuncs from "./lib/pkg/dnt_wasm.js";
import { source } from "./lib/pkg/dnt_wasm_bg.ts";

await init(source);

export interface TransformOptions {
  entryPoints: (string | URL)[];
  testEntryPoints?: (string | URL)[];
  shimPackageName: string;
  mappings?: SpecifierMappings;
}

/** Specifier to bare specifier mappings. */
export interface SpecifierMappings {
  [specifier: string]: {
    /** Name of the specifier to map to. */
    name: string;
    /** Version to use in the package.json file.
     *
     * Not specifying a version will exclude it from the package.json file.
     */
    version?: string;
  };
}

export interface Dependency {
  name: string;
  version: string;
}

export interface TransformOutput {
  main: TransformOutputEnvironment;
  test: TransformOutputEnvironment;
  warnings: string[];
}

export interface TransformOutputEnvironment {
  entryPoints: string[];
  /** If the shim is used in any of the output files. */
  shimUsed: boolean;
  dependencies: Dependency[];
  files: OutputFile[];
}

export interface OutputFile {
  filePath: string;
  fileText: string;
}

/** Analyzes the provided entry point to get all the dependended on modules and
 * outputs canonical TypeScript code in memory. The output of this function
 * can then be sent to the TypeScript compiler or a bundler for further processing. */
export function transform(options: TransformOptions): Promise<TransformOutput> {
  if (options.entryPoints.length === 0) {
    throw new Error("Specify one or more entry points.");
  }
  const newOptions = {
    ...options,
    mappings: options.mappings && Object.fromEntries(
      Object.entries(options.mappings).map(([key, value]) => {
        const lowerCaseKey = key.toLowerCase();
        if (
          !lowerCaseKey.startsWith("http://") &&
          !lowerCaseKey.startsWith("https://")
        ) {
          key = path.toFileUrl(lowerCaseKey).toString();
        }
        return [key, value];
      }),
    ),
    entryPoints: options.entryPoints.map(stringOrUrlToString),
    testEntryPoints: (options.testEntryPoints ?? []).map(stringOrUrlToString),
  };
  return wasmFuncs.transform(newOptions);
}

function stringOrUrlToString(value: string | URL) {
  if (value instanceof URL) {
    return value.toString();
  } else {
    return path.toFileUrl(path.resolve(value)).toString();
  }
}
