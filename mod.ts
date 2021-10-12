// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { outputDiagnostics } from "./lib/compiler.ts";
import { createProjectSync, path, ts } from "./lib/mod.deps.ts";
import { PackageJsonObject } from "./lib/types.ts";
import { transform } from "./transform.ts";

export * from "./transform.ts";

export interface BuildOptions {
  outDir: string;
  typeCheck?: boolean;
  entryPoint: string | URL;
  shimPackage?: {
    name: string;
    version: string;
  };
  /** Specifiers to map from and to. */
  mappings?: {
    [specifier: string]: {
      /** Name of the specifier to map to. */
      name: string;
      /** Version to use in the package.json file.
       *
       * Not specifying a version will exclude it from the package.json file.
       */
      version?: string;
    };
  };
  package: PackageJsonObject;
  writeFile?: (filePath: string, text: string) => void;
}

/** Emits the specified Deno module to an npm package using the TypeScript compiler. */
export async function build(options: BuildOptions): Promise<void> {
  await Deno.permissions.request({ name: "write", path: options.outDir });

  const shimPackage = options.shimPackage ?? {
    name: "deno.ns",
    version: "0.4.3",
  };
  const specifierMappings = options.mappings && Object.fromEntries(
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
  );

  console.log("Transforming...");
  const transformOutput = await transform({
    entryPoint: options.entryPoint,
    shimPackageName: shimPackage.name,
    specifierMappings: specifierMappings && Object.fromEntries(
      Object.entries(specifierMappings).map(([key, value]) => {
        return [key, value.name];
      }),
    ),
  });
  const createdDirectories = new Set<string>();
  const writeFile = options.writeFile ??
    ((filePath: string, fileText: string) => {
      const dir = path.dirname(filePath);
      if (!createdDirectories.has(dir)) {
        Deno.mkdirSync(dir, { recursive: true });
        createdDirectories.add(dir);
      }
      Deno.writeTextFileSync(filePath, fileText);
    });

  console.log("Running npm install...");
  createPackageJson();
  // npm install in order to prepare for checking TS diagnostics
  await npmInstall();

  console.log("Building TypeScript project...");
  const esmOutDir = path.join(options.outDir, "esm");
  const cjsOutDir = path.join(options.outDir, "cjs");
  const typesOutDir = path.join(options.outDir, "types");
  const project = createProjectSync({
    compilerOptions: {
      outDir: typesOutDir,
      allowJs: true,
      stripInternal: true,
      declaration: true,
      esModuleInterop: false,
      isolatedModules: true,
      useDefineForClassFields: true,
      experimentalDecorators: true,
      jsx: ts.JsxEmit.React,
      jsxFactory: "React.createElement",
      jsxFragmentFactory: "React.Fragment",
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
      module: ts.ModuleKind.ES2015,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      target: ts.ScriptTarget.ES2015,
      allowSyntheticDefaultImports: true,
    },
  });

  for (const outputFile of transformOutput.files) {
    project.createSourceFile(
      path.join(options.outDir, "src", outputFile.filePath),
      outputFile.fileText,
    );
  }

  let program = project.createProgram();

  if (options.typeCheck) {
    console.log("Type checking...");
    const diagnostics = ts.getPreEmitDiagnostics(program);
    if (diagnostics.length > 0) {
      outputDiagnostics(diagnostics);
      Deno.exit(1);
    }
  }

  // emit only the .d.ts files
  console.log("Emitting declaration files...");
  emit({ onlyDtsFiles: true });

  // emit the esm files
  console.log("Emitting esm module...");
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

  // emit the cjs files
  console.log("Emitting cjs module...");
  project.compilerOptions.set({
    declaration: false,
    esModuleInterop: true,
    outDir: cjsOutDir,
    module: ts.ModuleKind.CommonJS,
  });
  program = project.createProgram();
  emit();
  writeFile(
    path.join(cjsOutDir, "package.json"),
    `{\n  "type": "commonjs"\n}\n`,
  );

  function emit(opts?: { onlyDtsFiles?: boolean }) {
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
    );

    if (emitResult.diagnostics.length > 0) {
      outputDiagnostics(emitResult.diagnostics);
      Deno.exit(1);
    }
  }

  function createPackageJson() {
    const entryPointPath = transformOutput
      .entryPointFilePath
      .replace(/\.ts$/i, ".js");
    const entryPointDtsFilePath = transformOutput
      .entryPointFilePath
      .replace(/\.ts$/i, ".d.ts");
    const packageJsonObj = {
      ...options.package,
      dependencies: {
        // add dependencies from transform
        ...Object.fromEntries(
          transformOutput.dependencies.map((d) => [d.name, d.version]),
        ),
        // add specifier mappings to dependencies
        ...(specifierMappings && Object.fromEntries(
          Object.values(specifierMappings)
            .filter((v) => v.version)
            .map((value) => [value.name, value.version]),
        )) ?? {},
        // add shim
        ...(transformOutput.shimUsed
          ? {
            [shimPackage.name]: shimPackage.version,
          }
          : {}),
        // override with specified dependencies
        ...(options.package.dependencies ?? {}),
      },
      module: options.package.module ?? `./esm/${entryPointPath}`,
      main: options.package.main ?? `./cjs/${entryPointPath}`,
      types: options.package.types ?? `./types/${entryPointDtsFilePath}`,
      exports: {
        ...(options.package.exports ?? {}),
        ".": {
          import: `./esm/${entryPointPath}`,
          require: `./cjs/${entryPointPath}`,
          types: options.package.types ?? `./types/${entryPointDtsFilePath}`,
          ...(options.package.exports?.["."] ?? {}),
        },
      },
    };
    writeFile(
      path.join(options.outDir, "package.json"),
      JSON.stringify(packageJsonObj, undefined, 2),
    );
  }

  async function npmInstall() {
    const cmd = getCmd();
    await Deno.permissions.request({ name: "run", command: cmd[0] });
    const process = Deno.run({
      cmd,
      cwd: options.outDir,
      stderr: "inherit",
      stdout: "inherit",
      stdin: "inherit",
    });

    try {
      const status = await process.status();
      if (!status.success) {
        throw new Error(`npm install failed with exit code ${status.code}`);
      }
    } finally {
      process.close();
    }

    function getCmd() {
      const args = ["npm", "install"];
      if (Deno.build.os === "windows") {
        return ["cmd", "/c", ...args];
      } else {
        return args;
      }
    }
  }
}
