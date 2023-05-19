// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import {
  getCompilerLibOption,
  getCompilerScriptTarget,
  getCompilerSourceMapOptions,
  getTopLevelAwaitLocation,
  type LibName,
  libNamesToCompilerOption,
  outputDiagnostics,
  type SourceMapOptions,
  transformCodeToTarget,
} from "./lib/compiler.ts";
import { colors, createProjectSync, path, ts } from "./lib/mod.deps.ts";
import { type ShimOptions, shimOptionsToTransformShims } from "./lib/shims.ts";
import { getNpmIgnoreText } from "./lib/npm_ignore.ts";
import type { PackageJsonObject, ScriptTarget } from "./lib/types.ts";
import { glob, runNpmCommand, standardizePath } from "./lib/utils.ts";
import {
  type SpecifierMappings,
  transform,
  type TransformOutput,
} from "./transform.ts";
import * as compilerTransforms from "./lib/compiler_transforms.ts";
import { getPackageJson } from "./lib/package_json.ts";
import { getTestRunnerCode } from "./lib/test_runner/get_test_runner_code.ts";

export type { PackageJsonObject } from "./lib/types.ts";
export type { LibName, SourceMapOptions } from "./lib/compiler.ts";
export type { ShimOptions } from "./lib/shims.ts";
export { emptyDir } from "./lib/mod.deps.ts";

export interface EntryPoint {
  /**
   * If the entrypoint is for an npm binary or export.
   * @default "export"
   */
  kind?: "bin" | "export";
  /** Name of the entrypoint in the "binary" or "exports". */
  name: string;
  /** Path to the entrypoint. */
  path: string;
}

export interface BuildOptions {
  /** Entrypoint(s) to the Deno module. Ex. `./mod.ts` */
  entryPoints: (string | EntryPoint)[];
  /** Directory to output to. */
  outDir: string;
  /** Shims to use. */
  shims: ShimOptions;
  /** Type check the output.
   * @default true
   */
  typeCheck?: boolean;
  /** Collect and run test files.
   * @default true
   */
  test?: boolean;
  /** Create declaration files.
   *
   * * `"inline"` - Emit declaration files beside the .js files in both
   *   the esm and cjs folders. This is the recommended option when publishing
   *   a dual ESM and CJS package to npm.
   * * `"separate"` - Emits declaration files to the `types` folder where both
   *   the ESM and CJS code share the same type declarations.
   * * `false` - Do not emit declaration files.
   * @default "inline"
   */
  declaration?: "inline" | "separate" | false;
  /** Include a CommonJS or UMD module.
   * @default "cjs"
   */
  scriptModule?: "cjs" | "umd" | false;
  /** Whether to emit an ES module.
   * @default true
   */
  esModule?: boolean;
  /** Skip outputting the canonical TypeScript in the output directory before emitting.
   * @default false
   */
  skipSourceOutput?: boolean;
  /** Root directory to find test files in. Defaults to the cwd. */
  rootTestDir?: string;
  /** Glob pattern to use to find tests files. Defaults to `deno test`'s pattern. */
  testPattern?: string;
  /**
   * Specifiers to map from and to.
   *
   * This can be used to create a node specific file:
   *
   * ```
   * mappings: {
   *   "./file.deno.ts": "./file.node.ts",
   * }
   * ```
   *
   * Or map a specifier to an npm package:
   *
   * ```
   * mappings: {
   * "https://deno.land/x/code_block_writer@11.0.0/mod.ts": {
   *   name: "code-block-writer",
   *   version: "^11.0.0",
   * }
   * ```
   */
  mappings?: SpecifierMappings;
  /** Package.json output. You may override dependencies and dev dependencies in here. */
  package: PackageJsonObject;
  /** Path or url to import map. */
  importMap?: string;
  /** Package manager used to install dependencies and run npm scripts.
   * This also can be an absolute path to the executable file of package manager.
   * @default "npm"
   */
  packageManager?: "npm" | "yarn" | "pnpm" | string;
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
    /** Default set of library options to use. See https://www.typescriptlang.org/tsconfig/#lib */
    lib?: LibName[];
    /**
     * Skip type checking of declaration files (those in dependencies).
     * @default true
     */
    skipLibCheck?: boolean;
    /**
     * @default false
     */
    emitDecoratorMetadata?: boolean;
  };
  /** Action to do after emitting and before running tests. */
  postBuild?: () => void | Promise<void>;
}

/** Builds the specified Deno module to an npm package using the TypeScript compiler. */
export async function build(options: BuildOptions): Promise<void> {
  if (options.scriptModule === false && options.esModule === false) {
    throw new Error("`scriptModule` and `esModule` cannot both be `false`");
  }
  // set defaults
  options = {
    ...options,
    outDir: standardizePath(options.outDir),
    entryPoints: options.entryPoints,
    scriptModule: options.scriptModule ?? "cjs",
    esModule: options.esModule ?? true,
    typeCheck: options.typeCheck ?? true,
    test: options.test ?? true,
    declaration: (options.declaration as boolean) === true
      ? "inline"
      : options.declaration ?? "inline",
  };
  const packageManager = options.packageManager ?? "npm";
  const scriptTarget = options.compilerOptions?.target ?? "ES2021";
  const entryPoints: EntryPoint[] = options.entryPoints.map((e, i) => {
    if (typeof e === "string") {
      return {
        name: i === 0 ? "." : e.replace(/\.tsx?$/i, ".js"),
        path: standardizePath(e),
      };
    } else {
      return {
        ...e,
        path: standardizePath(e.path),
      };
    }
  });

  await Deno.permissions.request({ name: "write", path: options.outDir });

  log("Transforming...");
  const transformOutput = await transformEntryPoints();
  for (const warning of transformOutput.warnings) {
    warn(warning);
  }

  const createdDirectories = new Set<string>();
  const writeFile = (filePath: string, fileText: string) => {
    const dir = path.dirname(filePath);
    if (!createdDirectories.has(dir)) {
      Deno.mkdirSync(dir, { recursive: true });
      createdDirectories.add(dir);
    }
    Deno.writeTextFileSync(filePath, fileText);
  };

  createPackageJson();
  createNpmIgnore();

  // install dependencies in order to prepare for checking TS diagnostics
  log(`Running ${packageManager} install...`);
  const npmInstallPromise = runNpmCommand({
    bin: packageManager,
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
  const scriptOutDir = path.join(options.outDir, "script");
  const typesOutDir = path.join(options.outDir, "types");
  const compilerScriptTarget = getCompilerScriptTarget(scriptTarget);
  const project = createProjectSync({
    compilerOptions: {
      outDir: typesOutDir,
      allowJs: true,
      alwaysStrict: true,
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
      noStrictGenericChecks: false,
      noUncheckedIndexedAccess: false,
      declaration: !!options.declaration,
      esModuleInterop: false,
      isolatedModules: true,
      useDefineForClassFields: true,
      experimentalDecorators: true,
      emitDecoratorMetadata: options.compilerOptions?.emitDecoratorMetadata ??
        false,
      jsx: ts.JsxEmit.React,
      jsxFactory: "React.createElement",
      jsxFragmentFactory: "React.Fragment",
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
      module: ts.ModuleKind.ESNext,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      target: compilerScriptTarget,
      lib: libNamesToCompilerOption(
        options.compilerOptions?.lib ?? getCompilerLibOption(scriptTarget),
      ),
      allowSyntheticDefaultImports: true,
      importHelpers: options.compilerOptions?.importHelpers,
      ...getCompilerSourceMapOptions(options.compilerOptions?.sourceMap),
      inlineSources: options.compilerOptions?.inlineSources,
      skipLibCheck: options.compilerOptions?.skipLibCheck ?? true,
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

    if (options.scriptModule) {
      // cjs does not support TLA so error fast if we find one
      const tlaLocation = getTopLevelAwaitLocation(sourceFile);
      if (tlaLocation) {
        warn(
          `Top level await cannot be used when distributing CommonJS/UMD ` +
            `(See ${outputFile.filePath} ${tlaLocation.line + 1}:${
              tlaLocation.character + 1
            }). ` +
            `Please re-organize your code to not use a top level await or only distribute an ES module by setting the 'scriptModule' build option to false.`,
        );
        throw new Error(
          "Build failed due to top level await when creating CommonJS/UMD package.",
        );
      }
    }

    if (!options.skipSourceOutput) {
      writeFile(outputFilePath, outputFileText);
    }
  }

  // When creating the program and type checking, we need to ensure that
  // the cwd is the directory that contains the node_modules directory
  // so that TypeScript will read it and resolve any @types/ packages.
  // This is done in `getAutomaticTypeDirectiveNames` of TypeScript's code.
  const originalDir = Deno.cwd();
  let program: ts.Program;
  Deno.chdir(options.outDir);
  try {
    program = project.createProgram();

    if (options.typeCheck) {
      log("Type checking...");
      const diagnostics = ts.getPreEmitDiagnostics(program);
      if (diagnostics.length > 0) {
        outputDiagnostics(diagnostics);
        throw new Error(`Had ${diagnostics.length} diagnostics.`);
      }
    }
  } finally {
    Deno.chdir(originalDir);
  }

  // emit only the .d.ts files
  if (options.declaration === "separate") {
    log("Emitting declaration files...");
    emit({ onlyDtsFiles: true });
  }

  if (options.esModule) {
    // emit the esm files
    log("Emitting ESM package...");
    project.compilerOptions.set({
      declaration: options.declaration === "inline",
      outDir: esmOutDir,
    });
    program = project.createProgram();
    emit();
    writeFile(
      path.join(esmOutDir, "package.json"),
      `{\n  "type": "module"\n}\n`,
    );
  }

  // emit the script files
  if (options.scriptModule) {
    log("Emitting script package...");
    project.compilerOptions.set({
      declaration: options.declaration === "inline",
      esModuleInterop: true,
      outDir: scriptOutDir,
      module: options.scriptModule === "umd"
        ? ts.ModuleKind.UMD
        : ts.ModuleKind.CommonJS,
    });
    program = project.createProgram();
    emit({
      transformers: {
        before: [compilerTransforms.transformImportMeta],
      },
    });
    writeFile(
      path.join(scriptOutDir, "package.json"),
      `{\n  "type": "commonjs"\n}\n`,
    );
  }

  // ensure this is done before running tests
  await npmInstallPromise;

  // run post build action
  if (options.postBuild) {
    log("Running post build action...");
    await options.postBuild();
  }

  if (options.test) {
    log("Running tests...");
    createTestLauncherScript();
    await runNpmCommand({
      bin: packageManager,
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
      throw new Error(`Had ${emitResult.diagnostics.length} emit diagnostics.`);
    }
  }

  function createPackageJson() {
    const packageJsonObj = getPackageJson({
      entryPoints,
      transformOutput,
      package: options.package,
      testEnabled: options.test,
      includeEsModule: options.esModule !== false,
      includeScriptModule: options.scriptModule !== false,
      includeDeclarations: options.declaration === "separate",
      includeTsLib: options.compilerOptions?.importHelpers,
      shims: options.shims,
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
      includeScriptModule: options.scriptModule !== false,
      includeEsModule: options.esModule !== false,
    });
    writeFile(
      path.join(options.outDir, ".npmignore"),
      fileText,
    );
  }

  async function transformEntryPoints(): Promise<TransformOutput> {
    const { shims, testShims } = shimOptionsToTransformShims(options.shims);
    return transform({
      entryPoints: entryPoints.map((e) => e.path),
      testEntryPoints: options.test
        ? await glob({
          pattern: getTestPattern(),
          rootDir: options.rootTestDir ?? Deno.cwd(),
          excludeDirs: [options.outDir],
        })
        : [],
      shims,
      testShims,
      mappings: options.mappings,
      target: scriptTarget,
      importMap: options.importMap,
    });
  }

  function log(message: string) {
    console.log(`[dnt] ${message}`);
  }

  function warn(message: string) {
    console.warn(colors.yellow(`[dnt] ${message}`));
  }

  function createTestLauncherScript() {
    const denoTestShimPackage = getDependencyByName("@deno/shim-deno-test") ??
      getDependencyByName("@deno/shim-deno");
    writeFile(
      path.join(options.outDir, "test_runner.js"),
      transformCodeToTarget(
        getTestRunnerCode({
          denoTestShimPackageName: denoTestShimPackage == null
            ? undefined
            : denoTestShimPackage.name === "@deno/shim-deno"
            ? "@deno/shim-deno/test-internals"
            : denoTestShimPackage.name,
          testEntryPoints: transformOutput.test.entryPoints,
          includeEsModule: options.esModule !== false,
          includeScriptModule: options.scriptModule !== false,
        }),
        compilerScriptTarget,
      ),
    );

    function getDependencyByName(name: string) {
      return transformOutput.test.dependencies.find((d) => d.name === name) ??
        transformOutput.main.dependencies.find((d) => d.name === name);
    }
  }

  function getTestPattern() {
    // * named `test.{ts, tsx, js, mjs, jsx}`,
    // * or ending with `.test.{ts, tsx, js, mjs, jsx}`,
    // * or ending with `_test.{ts, tsx, js, mjs, jsx}`
    return options.testPattern ??
      "**/{test.{ts,tsx,js,mjs,jsx},*.test.{ts,tsx,js,mjs,jsx},*_test.{ts,tsx,js,mjs,jsx}}";
  }
}
