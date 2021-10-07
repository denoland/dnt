// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { outputDiagnostics } from "./lib/compiler.ts";
import { createProjectSync, path, ts } from "./lib/mod.deps.ts";
import { PackageJsonObject } from "./lib/types.ts";
import { OutputFile, transform } from "./transform.ts";

export * from "./transform.ts";

export interface RunOptions {
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
    }
  }
  package: PackageJsonObject;
  writeFile?: (filePath: string, text: string) => void;
}

/** Emits the specified Deno module to JavaScript code using the TypeScript compiler. */
export async function run(options: RunOptions): Promise<void> {
  const shimPackage = options.shimPackage ?? {
    name: "deno.ns",
    version: "0.4.3",
  };
  const specifierMappings = options.mappings && Object.fromEntries(
    Object.entries(options.mappings).map(([key, value]) => {
      const lowerCaseKey = key.toLowerCase();
      if (!lowerCaseKey.startsWith("http://") && !lowerCaseKey.startsWith("https://")) {
        key = path.toFileUrl(lowerCaseKey).toString();
      }
      return [key, value];
    }),
  )
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

  createPackageJson();
  // npm install in order to get diagnostics
  await npmInstall();

  // todo: use two workers for this
  const cjsResult = emitFiles({
    outDir: options.outDir,
    isCjs: true,
    outputFiles: transformOutput.cjsFiles,
    typeCheck: options.typeCheck ?? false,
    writeFile,
  });
  if (!cjsResult.success) {
    outputDiagnostics(cjsResult.diagnostics);
    Deno.exit(1);
  }

  const mjsResult = emitFiles({
    outDir: options.outDir,
    isCjs: false,
    outputFiles: transformOutput.mjsFiles,
    typeCheck: false, // don't type check twice
    writeFile,
  });
  if (!mjsResult.success) {
    outputDiagnostics(mjsResult.diagnostics);
    Deno.exit(1);
  }

  function createPackageJson() {
    const entryPointPath = transformOutput
      .entryPointFilePath
      .replace(/\.ts$/i, ".js");
    const packageJsonObj = {
      ...options.package,
      dependencies: {
        // add dependencies from transform
        ...Object.fromEntries(transformOutput.dependencies.map(d => [d.name, d.version])),
        // add specifier mappings to dependencies
        ...(specifierMappings && Object.fromEntries(
          Object.values(specifierMappings)
            .filter(v => v.version)
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
      main: options.package.main ?? `cjs/${entryPointPath}`,
      module: options.package.module ?? `mjs/${entryPointPath}`,
      exports: {
        ...(options.package.exports ?? {}),
        ".": {
          "import": `./mjs/${entryPointPath}`,
          "require": `./cjs/${entryPointPath}`,
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
    const cmd = Deno.run({
      cmd: getCmdArgs(),
      cwd: options.outDir,
      stderr: "inherit",
      stdout: "inherit",
      stdin: "inherit",
    });

    try {
      const status = await cmd.status();
      if (!status.success) {
        throw new Error(`npm install failed with exit code ${status.code}`);
      }
    } finally {
      cmd.close();
    }

    function getCmdArgs() {
      const args = ["npm", "install"];
      if (Deno.build.os === "windows") {
        return ["cmd", "/c", ...args];
      } else {
        return args;
      }
    }
  }
}

function emitFiles(options: {
  outDir: string;
  outputFiles: OutputFile[];
  isCjs: boolean;
  typeCheck: boolean;
  writeFile: ((filePath: string, text: string) => void);
}) {
  const moduleOutDir = path.join(options.outDir, options.isCjs ? "cjs" : "mjs");
  const project = createProjectSync({
    compilerOptions: {
      outDir: moduleOutDir,
      allowJs: true,
      stripInternal: true,
      declaration: true,
      esModuleInterop: options.isCjs,
      module: options.isCjs ? ts.ModuleKind.CommonJS : ts.ModuleKind.ES2015,
      target: ts.ScriptTarget.ES2015,
    },
  });

  for (const outputFile of options.outputFiles) {
    project.createSourceFile(path.join(options.outDir, "src", outputFile.filePath), outputFile.fileText);
  }

  const program = project.createProgram();

  if (options.typeCheck) {
    const diagnostics = ts.getPreEmitDiagnostics(program);
    if (diagnostics.length > 0) {
      return {
        success: false,
        diagnostics: [...diagnostics],
      };
    }
  }

  const emitResult = program.emit(
    undefined,
    (filePath, data, writeByteOrderMark) => {
      if (writeByteOrderMark) {
        data = "\uFEFF" + data;
      }
      options.writeFile(filePath, data);
    },
  );

  options.writeFile(
    path.join(moduleOutDir, "package.json"),
    `{\n  "type": "${options.isCjs ? "commonjs" : "module"}"\n}\n`,
  );

  return {
    success: emitResult.diagnostics.length === 0,
    diagnostics: [...emitResult.diagnostics],
  };
}
