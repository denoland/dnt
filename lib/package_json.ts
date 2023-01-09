// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import type { EntryPoint, ShimOptions } from "../mod.ts";
import { TransformOutput } from "../transform.ts";
import { PackageJsonObject } from "./types.ts";

export interface GetPackageJsonOptions {
  transformOutput: TransformOutput;
  entryPoints: EntryPoint[];
  package: PackageJsonObject;
  includeEsModule: boolean | undefined;
  includeScriptModule: boolean | undefined;
  includeDeclarations: boolean | undefined;
  includeTsLib: boolean | undefined;
  testEnabled: boolean | undefined;
  shims: ShimOptions;
}

export function getPackageJson({
  transformOutput,
  entryPoints,
  package: packageJsonObj,
  includeEsModule,
  includeScriptModule,
  includeDeclarations,
  includeTsLib,
  testEnabled,
  shims,
}: GetPackageJsonOptions): Record<string, unknown> {
  const finalEntryPoints = transformOutput
    .main.entryPoints.map((e, i) => ({
      name: entryPoints[i].name,
      kind: entryPoints[i].kind ?? "export",
      path: e.replace(/\.tsx?$/i, ".js"),
      types: e.replace(/\.tsx?$/i, ".d.ts"),
    }));
  const exports = finalEntryPoints.filter((e) => e.kind === "export");
  const binaries = finalEntryPoints.filter((e) => e.kind === "bin");
  const dependencies = {
    // typescript helpers library (https://www.npmjs.com/package/tslib)
    ...(includeTsLib
      ? {
        tslib: "^2.4.1",
      }
      : {}),
    // add dependencies from transform
    ...Object.fromEntries(
      transformOutput.main.dependencies
        .filter((d) => !d.peerDependency)
        .map((d) => [d.name, d.version]),
    ),
    // override with specified dependencies
    ...(packageJsonObj.dependencies ?? {}),
  };
  const peerDependencies = {
    // add dependencies from transform
    ...Object.fromEntries(
      transformOutput.main.dependencies
        .filter((d) => d.peerDependency)
        .map((d) => [d.name, d.version]),
    ),
    // override with specified dependencies
    ...(packageJsonObj.peerDependencies ?? {}),
  };
  const testDevDependencies = testEnabled
    ? ({
      ...(!Object.keys(dependencies).includes("chalk")
        ? {
          "chalk": "^4.1.2",
        }
        : {}),
      // add dependencies from transform
      ...Object.fromEntries(
        // ignore peer dependencies on this
        transformOutput.test.dependencies.map((d) => [d.name, d.version]) ??
          [],
      ),
    })
    : {};
  const devDependencies = {
    ...(shouldIncludeTypesNode()
      ? {
        "@types/node": "^18.11.9",
      }
      : {}),
    ...testDevDependencies,
    // override with specified dependencies
    ...(packageJsonObj.devDependencies ?? {}),
  };
  const scripts = testEnabled
    ? ({
      test: "node test_runner.js",
      // override with specified scripts
      ...(packageJsonObj.scripts ?? {}),
    })
    : packageJsonObj.scripts;
  const mainExport = exports.length > 0
    ? {
      module: includeEsModule ? `./esm/${exports[0].path}` : undefined,
      main: includeScriptModule ? `./script/${exports[0].path}` : undefined,
      types: includeDeclarations ? `./types/${exports[0].types}` : undefined,
    }
    : {};
  const binaryExport = binaries.length > 0
    ? {
      bin: Object.fromEntries(binaries.map((b) => [b.name, `./esm/${b.path}`])),
    }
    : {};

  return {
    ...mainExport,
    ...binaryExport,
    ...packageJsonObj,
    ...deleteEmptyKeys({
      exports: {
        ...(includeEsModule || exports.length > 1
          ? {
            ...(Object.fromEntries(exports.map((e) => {
              return [e.name, {
                import: includeEsModule
                  ? getPathOrTypesObject(`./esm/${e.path}`)
                  : undefined,
                require: includeScriptModule
                  ? getPathOrTypesObject(`./script/${e.path}`)
                  : undefined,
                ...(packageJsonObj.exports?.[e.name] ?? {}),
              }];

              function getPathOrTypesObject(path: string) {
                if (includeDeclarations) {
                  return {
                    // "types" must always be first and "default" last
                    types:
                      (e.name === "." ? packageJsonObj.types : undefined) ??
                        `./types/${e.types}`,
                    default: path,
                  };
                } else {
                  return path;
                }
              }
            }))),
          }
          : {}),
        // allow someone to override
        ...(packageJsonObj.exports ?? {}),
      },
      scripts,
      dependencies,
      peerDependencies,
      devDependencies,
    }),
  };

  function shouldIncludeTypesNode() {
    if (Object.keys(dependencies).includes("@types/node")) {
      return false;
    }

    if (typeof shims.deno === "object") {
      if (shims.deno.test) {
        return true;
      } else {
        return false;
      }
    } else if (shims.deno || shims.undici) {
      return true;
    } else {
      return false;
    }
  }

  function deleteEmptyKeys(obj: Record<string, unknown>) {
    for (const key of Object.keys(obj)) {
      const value = obj[key];
      if (
        typeof value === "object" && value != null &&
        Object.keys(value).length === 0
      ) {
        delete obj[key];
      }
    }
    return obj;
  }
}
