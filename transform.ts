// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { path } from "./lib/transform.deps.ts";
import init, * as wasmFuncs from "./lib/pkg/dnt_wasm.js";
import { source } from "./lib/pkg/dnt_wasm_bg.ts";

await init(source);

export interface Redirects {
  /** The to and from specifier redirect. */
  [specifier: string]: string;
}

/** Specifier to bare specifier mappings. */
export interface SpecifierMappings {
  [specifier: string]: MappedSpecifier;
}

export interface MappedSpecifier {
  /** Name of the npm package specifier to map to. */
  name: string;
  /** Version to use in the package.json file.
   *
   * Not specifying a version will exclude it from the package.json file.
   */
  version?: string;
}

export interface GlobalName {
  /** Name to use as the global name. */
  name: string;
  /** Name of the export from the package.
   * @remarks Defaults to the name. Specify `"default"` to use the default export.
   */
  exportName?: string;
  /** Whether this is a name that only exists as a type declaration. */
  typeOnly?: boolean;
}

export interface Shim {
  /** Information about the npm package or bare specifier to import. */
  package: MappedSpecifier;
  /** Named exports from the shim to use as globals. */
  globalNames: (GlobalName | string)[];
}

export interface TransformOptions {
  entryPoints: string[];
  testEntryPoints?: string[];
  shims?: Shim[];
  testShims?: Shim[];
  mappings?: SpecifierMappings;
  redirects?: Redirects;
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
    mappings: Object.fromEntries(
      Object.entries(options.mappings ?? {}).map(([key, value]) => {
        return [valueToUrl(key), value];
      }),
    ),
    redirects: Object.fromEntries(
      Object.entries(options.redirects ?? {}).map(([key, value]) => {
        return [valueToUrl(key), valueToUrl(value)];
      }),
    ),
    entryPoints: options.entryPoints.map(valueToUrl),
    testEntryPoints: (options.testEntryPoints ?? []).map(valueToUrl),
    shims: (options.shims ?? []).map(mapShim),
    testShims: (options.testShims ?? []).map(mapShim),
  };
  return wasmFuncs.transform(newOptions);
}

function valueToUrl(value: string) {
  const lowerCaseValue = value.toLowerCase();
  if (
    lowerCaseValue.startsWith("http://") ||
    lowerCaseValue.startsWith("https://")
  ) {
    return value;
  } else {
    return path.toFileUrl(path.resolve(value)).toString();
  }
}

function mapShim(value: Shim): Shim {
  return {
    ...value,
    globalNames: value.globalNames.map(mapToGlobalName),
  };
}

function mapToGlobalName(value: string | GlobalName): GlobalName {
  if (typeof value === "string") {
    return {
      name: value,
      typeOnly: false,
    };
  } else {
    value.typeOnly ??= false;
    return value;
  }
}
