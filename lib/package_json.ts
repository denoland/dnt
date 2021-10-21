// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import type { EntryPoint } from "../mod.ts";
import { TransformOutput } from "../transform.ts";
import { PackageJsonObject } from "./types.ts";

export interface GetPackageJsonOptions {
  transformOutput: TransformOutput;
  entryPoints: EntryPoint[];
  shimPackage: {
    name: string;
    version: string;
  };
  package: PackageJsonObject;
  testEnabled: boolean | undefined;
}

export function getPackageJson({
  transformOutput,
  entryPoints,
  shimPackage,
  package: packageJsonObj,
  testEnabled,
}: GetPackageJsonOptions) {
  const finalEntryPoints = transformOutput
    .main.entryPoints.map((e, i) => ({
      name: entryPoints[i].name,
      path: e.replace(/\.tsx?$/i, ".js"),
      types: e.replace(/\.tsx?$/i, ".d.ts"),
    }));
  const dependencies = {
    // add dependencies from transform
    ...Object.fromEntries(
      transformOutput.main.dependencies.map((d) => [d.name, d.version]),
    ),
    // add shim
    ...(transformOutput.main.shimUsed
      ? {
        [shimPackage.name]: shimPackage.version,
      }
      : {}),
    // override with specified dependencies
    ...(packageJsonObj.dependencies ?? {}),
  };
  const testDevDependencies = testEnabled
    ? ({
      ...(!Object.keys(dependencies).includes("chalk")
        ? {
          "chalk": "4.1.2",
        }
        : {}),
      // add dependencies from transform
      ...Object.fromEntries(
        transformOutput.test.dependencies.map((d) => [d.name, d.version]) ??
          [],
      ),
    })
    : {};
  const devDependencies = {
    ...(!Object.keys(dependencies).includes("@types/node") &&
        (transformOutput.main.shimUsed || transformOutput.test.shimUsed)
      ? {
        "@types/node": "16.11.1",
      }
      : {}),
    ...testDevDependencies,
    // add shim if not in dependencies
    ...(transformOutput.test.shimUsed &&
        !Object.keys(dependencies).includes(shimPackage.name)
      ? {
        [shimPackage.name]: shimPackage.version,
      }
      : {}),
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

  return {
    module: `./esm/${finalEntryPoints[0].path}`,
    main: `./umd/${finalEntryPoints[0].path}`,
    types: `./types/${finalEntryPoints[0].types}`,
    ...packageJsonObj,
    exports: {
      ...(packageJsonObj.exports ?? {}),
      ...(Object.fromEntries(finalEntryPoints.map((e) => [e.name, {
        import: `./esm/${e.path}`,
        require: `./umd/${e.path}`,
        types: (e.name === "." ? packageJsonObj.types : undefined) ??
          `./types/${e.types}`,
        ...(packageJsonObj.exports?.[e.name] ?? {}),
      }]))),
    },
    scripts,
    dependencies,
    devDependencies,
  };
}
