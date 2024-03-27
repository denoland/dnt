// Copyright 2018-2024 the Deno authors. MIT license.

/**
 * Lower level `transform` functionality that's used by the CLI
 * to convert Deno code to Node code.
 * @module
 */

import { instantiate } from "./lib/pkg/dnt_wasm.generated.js";
import type { ScriptTarget } from "./lib/types.ts";
import { valueToUrl } from "./lib/utils.ts";

/** Specifier to specifier mappings. */
export interface SpecifierMappings {
  /** Map a specifier to another module or npm package. */
  [specifier: string]: PackageMappedSpecifier | string;
}

export interface PackageMappedSpecifier {
  /** Name of the npm package specifier to map to. */
  name: string;
  /** Version to use in the package.json file.
   *
   * Not specifying a version will exclude it from the package.json file.
   * This is useful for built-in modules such as "fs".
   */
  version?: string;
  /** Sub path of the npm package to use in the module specifier.
   *
   * @remarks This should not include the package name and should not
   * include a leading slash. It will be concatenated to the package
   * name in the module specifier like so: `<package-name>/<sub-path>`
   */
  subPath?: string;
  /** If this should be a peer dependency. */
  peerDependency?: boolean;
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

export type Shim = PackageShim | ModuleShim;

export interface PackageShim {
  /** Information about the npm package specifier to import. */
  package: PackageMappedSpecifier;
  /** Npm package to include in the dev depedencies that has the type declarations. */
  typesPackage?: Dependency;
  /** Named exports from the shim to use as globals. */
  globalNames: (GlobalName | string)[];
}

export interface ModuleShim {
  /** The module or bare specifier. */
  module: string;
  /** Named exports from the shim to use as globals. */
  globalNames: (GlobalName | string)[];
}

export interface TransformOptions {
  entryPoints: string[];
  testEntryPoints?: string[];
  shims?: Shim[];
  testShims?: Shim[];
  mappings?: SpecifierMappings;
  target: ScriptTarget;
  /// Path or url to the import map.
  importMap?: string;
  internalWasmUrl?: string;
}

/** Dependency in a package.json file. */
export interface Dependency {
  /** Name of the package. */
  name: string;
  /** Version specifier (ex. `^1.0.0`). */
  version: string;
  /** If this is suggested to be a peer dependency. */
  peerDependency?: boolean;
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
export async function transform(
  options: TransformOptions,
): Promise<TransformOutput> {
  if (options.entryPoints.length === 0) {
    throw new Error("Specify one or more entry points.");
  }
  const newOptions = {
    ...options,
    mappings: Object.fromEntries(
      Object.entries(options.mappings ?? {}).map(([key, value]) => {
        return [valueToUrl(key), mapMappedSpecifier(value)];
      }),
    ),
    entryPoints: options.entryPoints.map(valueToUrl),
    testEntryPoints: (options.testEntryPoints ?? []).map(valueToUrl),
    shims: (options.shims ?? []).map(mapShim),
    testShims: (options.testShims ?? []).map(mapShim),
    target: options.target,
    importMap: options.importMap == null
      ? undefined
      : valueToUrl(options.importMap),
  };
  const wasmFuncs = await instantiate({
    url: options.internalWasmUrl ? new URL(options.internalWasmUrl) : undefined,
  });
  return wasmFuncs.transform(newOptions);
}

type SerializableMappedSpecifier = {
  kind: "package";
  value: PackageMappedSpecifier;
} | {
  kind: "module";
  value: string;
};

function mapMappedSpecifier(
  value: string | PackageMappedSpecifier,
): SerializableMappedSpecifier {
  if (typeof value === "string") {
    if (isPathOrUrl(value)) {
      return {
        kind: "module",
        value: valueToUrl(value),
      };
    } else {
      return {
        kind: "package",
        value: {
          name: value,
        },
      };
    }
  } else {
    return {
      kind: "package",
      value,
    };
  }
}

type SerializableShim = { kind: "package"; value: PackageShim } | {
  kind: "module";
  value: ModuleShim;
};

function mapShim(value: Shim): SerializableShim {
  const newValue: Shim = {
    ...value,
    globalNames: value.globalNames.map(mapToGlobalName),
  };
  if (isPackageShim(newValue)) {
    return { kind: "package", value: newValue };
  } else {
    return {
      kind: "module",
      value: {
        ...newValue,
        module: resolveBareSpecifierOrPath(newValue.module),
      },
    };
  }
}

function isPackageShim(value: Shim): value is PackageShim {
  return (value as PackageShim).package != null;
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

function resolveBareSpecifierOrPath(value: string) {
  value = value.trim();
  if (isPathOrUrl(value)) {
    return valueToUrl(value);
  } else {
    return value;
  }
}

function isPathOrUrl(value: string) {
  value = value.trim();
  return /^[a-z]+:\/\//i.test(value) || // has scheme
    value.startsWith("./") ||
    value.startsWith("../") ||
    /\.[a-z]+$/i.test(value); // has extension
}
