// Copyright 2018-2024 the Deno authors. MIT license.

/**
 * Lower level `transform` functionality that's used by the CLI
 * to convert Deno code to Node code.
 * @module
 */

import { existsSync } from "@std/fs";
import * as path from "@std/path";
import * as wasm from "./lib/pkg/dnt_wasm.js";
import type { PolyfillName, ScriptTarget } from "./lib/types.ts";
import { standardizePath, valueToUrl } from "./lib/utils.ts";

/** Specifier to specifier mappings. */
export interface SpecifierMappings {
  /** Map a specifier to another module or npm package.
   *
   * The specifier may be a path, url, or a bare specifier that's resolved
   * via the config file's import map (ex. `my-lib`).
   */
  [specifier: string]: PackageMappedSpecifier | string;
}

export interface PackageMappedSpecifier {
  /** Name of the npm package specifier to map to.
   *
   * @remarks An `@types/` package is imported by the name of the package it
   * provides the declarations for, since TypeScript errors when a
   * declaration file package is imported directly.
   */
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
  /** Entry points that are only used as an npm binary, which is a subset
   * of the entry points. */
  binEntryPoints?: string[];
  testEntryPoints?: string[];
  shims?: Shim[];
  testShims?: Shim[];
  mappings?: SpecifierMappings;
  target: ScriptTarget;
  /** Explicitly enables or disables polyfills by name, taking precedence
   * over what `target` implies.
   */
  polyfills?: Partial<Record<PolyfillName, boolean>>;
  /// Path or url to the import map.
  importMap?: string;
  /** Path or url to a deno.json.
   *
   * When not specified, a deno.json is auto-discovered by searching upwards
   * from the entry points.
   *
   * Specify `false` to disable the auto-discovery, which also disables
   * discovering a package.json and deno.lock.
   */
  configFile?: string | false;
  /**
   * Errors when the deno lock file would need to be updated in order to
   * transform (ex. a dependency is not in it).
   *
   * Leave this undefined to use the `lock.frozen` setting in the deno.json file.
   */
  frozenLockfile?: boolean;
  /** Path or file url to the directory that the relative paths in these
   * options resolve from and that a config file, `deno.lock`, and
   * `node_modules` directory are discovered relative to. */
  cwd: string;
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
  /** Path of the config file that was auto-discovered by searching upwards
   * from the entry points (or the cwd when there are no local entry points).
   *
   * This is `undefined` when no config file was found, when one was
   * explicitly provided, or when auto-discovery is disabled.
   */
  discoveredConfigFile?: string;
  /** Packages that provide the type declarations of a mapped dependency
   * (ex. an `@types/` package specified by an `X-TypeScript-Types` header).
   */
  typesDependencies: Dependency[];
  /** Output files that are only reachable from a binary entrypoint. */
  binOnlyFiles: string[];
}

export interface TransformOutputEnvironment {
  entryPoints: string[];
  dependencies: Dependency[];
  files: OutputFile[];
}

export interface OutputFile {
  filePath: string;
  fileText: string;
  bytes?: Uint8Array;
}

/** Analyzes the provided entry point to get all the dependended on modules and
 * outputs canonical TypeScript code in memory. The output of this function
 * can then be sent to the TypeScript compiler or a bundler for further processing. */
export function transform(
  options: TransformOptions,
): Promise<TransformOutput> {
  if (options.entryPoints.length === 0) {
    throw new Error("Specify one or more entry points.");
  }
  // all the relative paths in the options resolve from here
  const cwd = standardizePath(options.cwd, Deno.cwd());
  const newOptions = {
    ...options,
    mappings: Object.fromEntries(
      Object.entries(options.mappings ?? {}).map(([key, value]) => {
        return [mapMappingKey(key, cwd), mapMappedSpecifier(value, cwd)];
      }),
    ),
    entryPoints: options.entryPoints.map((e) => valueToUrl(e, cwd)),
    binEntryPoints: (options.binEntryPoints ?? []).map((e) =>
      valueToUrl(e, cwd)
    ),
    testEntryPoints: (options.testEntryPoints ?? []).map((e) =>
      valueToUrl(e, cwd)
    ),
    shims: (options.shims ?? []).map((s) => mapShim(s, cwd)),
    testShims: (options.testShims ?? []).map((s) => mapShim(s, cwd)),
    target: options.target,
    polyfills: options.polyfills ?? {},
    importMap: options.importMap == null
      ? undefined
      : valueToUrl(options.importMap, cwd),
    configFile: typeof options.configFile === "string"
      ? valueToUrl(options.configFile, cwd)
      : undefined,
    noConfig: options.configFile === false,
    cwd: path.toFileUrl(cwd).toString(),
  };
  return wasm.transform(newOptions);
}

function mapMappingKey(key: string, cwd: string) {
  key = key.trim();
  if (/^[a-z]+:/i.test(key) || isRelativeOrAbsolutePath(key)) {
    return valueToUrl(key, cwd);
  }
  // fall back to a path for a key like `mod.ts` that resolved to
  // one before bare specifiers were supported
  if (existsSync(path.resolve(cwd, key))) {
    return valueToUrl(key, cwd);
  }
  // leave bare specifiers alone so that they're resolved
  // via the config file's import map (ex. `my-lib`)
  return key;
}

function isRelativeOrAbsolutePath(value: string) {
  return /^\.\.?[\\/]/.test(value) || path.isAbsolute(value);
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
  cwd: string,
): SerializableMappedSpecifier {
  if (typeof value === "string") {
    if (isPathOrUrl(value)) {
      return {
        kind: "module",
        value: valueToUrl(value, cwd),
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

function mapShim(value: Shim, cwd: string): SerializableShim {
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
        module: resolveBareSpecifierOrPath(newValue.module, cwd),
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

function resolveBareSpecifierOrPath(value: string, cwd: string) {
  value = value.trim();
  if (isPathOrUrl(value)) {
    return valueToUrl(value, cwd);
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
