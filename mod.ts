// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import {
  getCompilerScriptTarget,
  getCompilerSourceMapOptions,
  getTopLevelAwait,
  outputDiagnostics,
  SourceMapOptions,
} from "./lib/compiler.ts";
import { colors, createProjectSync, path, ts } from "./lib/mod.deps.ts";
import { getNpmIgnoreText } from "./lib/npm_ignore.ts";
import { PackageJsonObject } from "./lib/types.ts";
import { glob, runNpmCommand } from "./lib/utils.ts";
import { SpecifierMappings, transform, TransformOutput } from "./transform.ts";
import * as compilerTransforms from "./lib/compiler_transforms.ts";
import { getPackageJson } from "./lib/package_json.ts";
import { ScriptTarget } from "./lib/compiler.ts";
import { getTestRunnerCode } from "./lib/test_runner/get_test_runner_code.ts";

export interface EntryPoint {
  /**
   * If the entrypoint is for an npm binary or export.
   * @default "export"
   */
  kind?: "bin" | "export";
  /** Name of the entrypoint in the "binary" or "exports". */
  name: string;
  /** Path to the entrypoint. */
  path: string | URL;
}

export interface BuildOptions {
  /** Entrypoint(s) to the Deno module. Ex. `./mod.ts` */
  entryPoints: (string | EntryPoint)[];
  /** Directory to output to. */
  outDir: string;
  /** Type check the output.
   * @default true
   */
  typeCheck?: boolean;
  /** Collect and run test files.
   * @default true
   */
  test?: boolean;
  /** Create declaration files.
   * @default true
   */
  declaration?: boolean;
  /** Include a CommonJS module.
   * @default true
   */
  cjs?: boolean;
  /** Skip outputting the canonical TypeScript in the output directory before emitting.
   * @default false
   */
  skipSourceOutput?: boolean;
  /** Root directory to find test files in. Defaults to the cwd. */
  rootTestDir?: string;
  /** Glob pattern to use to find tests files. Defaults to `deno test`'s pattern. */
  testPattern?: string;
  /** Package to use for shimming the `Deno` namespace. Defaults to `deno.ns` */
  shimPackage?: {
    name: string;
    version: string;
  };
  /** Specifiers to map from and to. */
  mappings?: SpecifierMappings;
  /** Package.json output. You may override dependencies and dev dependencies in here. */
  package: PackageJsonObject;
  /** Optional compiler options. */
  compilerOptions?: {
    /** Uses tslib to import helper functions once per project instead of including them per-file if necessary.
     * @default false
     */
    importHelpers?: boolean;
    target?: ScriptTarget;
    /**
     * Use source maps from the canonical typescript to ESM/CommonJS emit.
     *
     * Specify `true` to include separate files or `"inline"` to inline the source map in the same file.
     * @remarks Using this option will cause your sources to be included in the npm package.
     * @default false
     */
    sourceMap?: SourceMapOptions;
    /**
     * Whether to include the source file text in the source map when using source maps.
     * @remarks It's not recommended to do this if you are distributing both ESM and CommonJS
     * sources as then it will duplicate the the source data being published.
     */
    inlineSources?: boolean;
  };
}

/** Emits the specified Deno module to an npm package using the TypeScript compiler. */
export async function build(options: BuildOptions): Promise<void> {
  // set defaults
  options = {
    ...options,
    cjs: options.cjs ?? true,
    typeCheck: options.typeCheck ?? true,
    test: options.test ?? true,
    declaration: options.declaration ?? true,
  };
  const entryPoints: EntryPoint[] = options.entryPoints.map((e, i) => {
    if (typeof e === "string") {
      return {
        name: i === 0 ? "." : e.replace(/\.tsx?$/i, ".js"),
        path: e,
      };
    } else {
      return e;
    }
  });

  await Deno.permissions.request({ name: "write", path: options.outDir });

  const shimPackage = options.shimPackage ?? {
    name: "deno.ns",
    version: "0.7.0",
  };

  log("Transforming...");
  const transformOutput = await transformEntryPoints();
  for (const warning of transformOutput.warnings) {
    warn(warning);
  }

  const createdDirectories = new Set<string>();
  const writeFile = ((filePath: string, fileText: string) => {
    const dir = path.dirname(filePath);
    if (!createdDirectories.has(dir)) {
      Deno.mkdirSync(dir, { recursive: true });
      createdDirectories.add(dir);
    }
    Deno.writeTextFileSync(filePath, fileText);
  });

  createPackageJson();
  createNpmIgnore();

  // npm install in order to prepare for checking TS diagnostics
  log("Running npm install...");
  const npmInstallPromise = runNpmCommand({
    args: ["install"],
    cwd: options.outDir,
  });
  if (options.typeCheck || options.declaration) {
    // Unfortunately this can't be run in parallel to building the project
    // in this case because TypeScript will resolve the npm packages when
    // creating the project.
    await npmInstallPromise;
  }

  log("Building project...");
  const esmOutDir = path.join(options.outDir, "esm");
  const umdOutDir = path.join(options.outDir, "umd");
  const typesOutDir = path.join(options.outDir, "types");
  const project = createProjectSync({
    compilerOptions: {
      outDir: typesOutDir,
      allowJs: true,
      stripInternal: true,
      strictBindCallApply: true,
      strictFunctionTypes: true,
      strictNullChecks: true,
      strictPropertyInitialization: true,
      suppressExcessPropertyErrors: false,
      suppressImplicitAnyIndexErrors: false,
      noImplicitAny: true,
      noImplicitReturns: false,
      noImplicitThis: true,
      noImplicitUseStrict: true,
      noStrictGenericChecks: false,
      noUncheckedIndexedAccess: false,
      declaration: options.declaration,
      esModuleInterop: false,
      isolatedModules: true,
      useDefineForClassFields: true,
      experimentalDecorators: true,
      jsx: ts.JsxEmit.React,
      jsxFactory: "React.createElement",
      jsxFragmentFactory: "React.Fragment",
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      target: getCompilerScriptTarget(options.compilerOptions?.target),
      allowSyntheticDefaultImports: true,
      importHelpers: options.compilerOptions?.importHelpers,
      ...getCompilerSourceMapOptions(options.compilerOptions?.sourceMap),
      inlineSources: options.compilerOptions?.inlineSources,
    },
  });

  const binaryEntryPointPaths = new Set(
    entryPoints.map((e, i) => ({
      kind: e.kind,
      path: transformOutput.main.entryPoints[i],
    })).filter((p) => p.kind === "bin").map((p) => p.path),
  );

  for (
    const outputFile of [
      ...transformOutput.main.files,
      ...transformOutput.test.files,
    ]
  ) {
    const outputFilePath = path.join(
      options.outDir,
      "src",
      outputFile.filePath,
    );
    const outputFileText = binaryEntryPointPaths.has(outputFile.filePath)
      ? `#!/usr/bin/env node\n${outputFile.fileText}`
      : outputFile.fileText;
    const sourceFile = project.createSourceFile(
      outputFilePath,
      outputFileText,
    );

    if (options.cjs) {
      // cjs does not support TLA so error fast if we find one
      const tlaLocation = getTopLevelAwait(sourceFile);
      if (tlaLocation) {
        warn(
          `Top level await cannot be used when distributing CommonJS ` +
            `(See ${outputFile.filePath} ${tlaLocation.line + 1}:${
              tlaLocation.character + 1
            }). ` +
            `Please re-organize your code to not use a top level await or only distribute an ESM module by setting the 'cjs' build option to false.`,
        );
        throw new Error(
          "Build failed due to top level await when creating CommonJS package.",
        );
      }
    }

    if (!options.skipSourceOutput) {
      writeFile(outputFilePath, outputFileText);
    }
  }

  let program = project.createProgram();

  if (options.typeCheck) {
    log("Type checking...");
    const diagnostics = ts.getPreEmitDiagnostics(program);
    if (diagnostics.length > 0) {
      outputDiagnostics(diagnostics);
      Deno.exit(1);
    }
  }

  // emit only the .d.ts files
  if (options.declaration) {
    log("Emitting declaration files...");
    emit({ onlyDtsFiles: true });
  }

  // emit the esm files
  log("Emitting ESM package...");
  project.compilerOptions.set({
    declaration: false,
    outDir: esmOutDir,
  });
  program = project.createProgram();
  emit();
  writeFile(
    path.join(esmOutDir, "package.json"),
    `{\n  "type": "module"\n}\n`,
  );

  // emit the umd files
  if (options.cjs) {
    log("Emitting CommonJS package...");
    project.compilerOptions.set({
      declaration: false,
      esModuleInterop: true,
      outDir: umdOutDir,
      module: ts.ModuleKind.UMD,
    });
    program = project.createProgram();
    emit({
      transformers: {
        before: [compilerTransforms.transformImportMeta],
      },
    });
    writeFile(
      path.join(umdOutDir, "package.json"),
      `{\n  "type": "commonjs"\n}\n`,
    );
  }

  // ensure this is done before running tests
  await npmInstallPromise;

  if (options.test) {
    log("Running tests...");
    createTestLauncherScript();
    await runNpmCommand({
      args: ["run", "test"],
      cwd: options.outDir,
    });
  }

  log("Complete!");

  function emit(
    opts?: { onlyDtsFiles?: boolean; transformers?: ts.CustomTransformers },
  ) {
    const emitResult = program.emit(
      undefined,
      (filePath, data, writeByteOrderMark) => {
        if (writeByteOrderMark) {
          data = "\uFEFF" + data;
        }
        writeFile(filePath, data);
      },
      undefined,
      opts?.onlyDtsFiles,
      opts?.transformers,
    );

    if (emitResult.diagnostics.length > 0) {
      outputDiagnostics(emitResult.diagnostics);
      Deno.exit(1);
    }
  }

  function createPackageJson() {
    const packageJsonObj = getPackageJson({
      entryPoints,
      shimPackage,
      transformOutput,
      package: options.package,
      testEnabled: options.test,
      includeCjs: options.cjs,
      includeDeclarations: options.declaration,
      includeTsLib: options.compilerOptions?.importHelpers,
    });
    writeFile(
      path.join(options.outDir, "package.json"),
      JSON.stringify(packageJsonObj, undefined, 2),
    );
  }

  function createNpmIgnore() {
    const fileText = getNpmIgnoreText({
      sourceMap: options.compilerOptions?.sourceMap,
      inlineSources: options.compilerOptions?.inlineSources,
      testFiles: transformOutput.test.files,
    });
    writeFile(
      path.join(options.outDir, ".npmignore"),
      fileText,
    );
  }

  async function transformEntryPoints(): Promise<TransformOutput> {
    return transform({
      entryPoints: entryPoints.map((e) => e.path),
      testEntryPoints: options.test
        ? await glob({
          pattern: getTestPattern(),
          rootDir: options.rootTestDir ?? Deno.cwd(),
          excludeDirs: [options.outDir],
        })
        : [],
      shimPackageName: shimPackage.name,
      mappings: options.mappings,
    });
  }

  function log(message: string) {
    console.log(`[dnt] ${message}`);
  }

  function warn(message: string) {
    console.warn(colors.yellow(`[dnt] ${message}`));
  }

  function createTestLauncherScript() {
    writeFile(
      path.join(options.outDir, "test_runner.js"),
      getTestRunnerCode({
        shimPackageName: shimPackage.name,
        testEntryPoints: transformOutput.test.entryPoints,
        testShimUsed: transformOutput.test.shimUsed,
        includeCjs: options.cjs,
      }),
    );
  }

  function getTestPattern() {
    // * named `test.{ts, tsx, js, mjs, jsx}`,
    // * or ending with `.test.{ts, tsx, js, mjs, jsx}`,
    // * or ending with `_test.{ts, tsx, js, mjs, jsx}`
    return options.testPattern ??
      "**/{test.{ts,tsx,js,mjs,jsx},*.test.{ts,tsx,js,mjs,jsx},*_test.{ts,tsx,js,mjs,jsx}}";
  }
}
