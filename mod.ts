// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { createProjectSync, path, ts } from "./lib/mod.deps.ts";
import { PackageJsonObject } from "./lib/types.ts";
import { OutputFile, transform } from "./transform.ts";

export * from "./transform.ts";

export interface EmitOptions {
  outDir: string;
  typeCheck?: boolean;
  entryPoint: string | URL;
  shimPackageName?: string;
  package: PackageJsonObject;
  writeFile?: (filePath: string, text: string) => void;
}

export interface EmitResult {
  success: boolean;
  diagnostics: ts.Diagnostic[];
}

/** Emits the specified Deno module to JavaScript code using the TypeScript compiler. */
export async function emit(options: EmitOptions): Promise<EmitResult> {
  const transformOutput = await transform({
    entryPoint: options.entryPoint,
    shimPackageName: options.shimPackageName,
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
  // todo: use two workers for this
  const cjsResult = emitFiles({
    outDir: path.join(options.outDir, "cjs"),
    isCjs: true,
    outputFiles: transformOutput.cjsFiles,
    typeCheck: options.typeCheck ?? false,
    writeFile,
  });
  if (!cjsResult.success) {
    return cjsResult;
  }
  const mjsResult = emitFiles({
    outDir: path.join(options.outDir, "mjs"),
    isCjs: false,
    outputFiles: transformOutput.mjsFiles,
    typeCheck: false, // don't type check twice
    writeFile,
  });
  if (!mjsResult.success) {
    return mjsResult;
  }

  const entryPointPath = transformOutput.entryPointFilePath.replace(/\.ts$/i, ".js");
  const packageJsonObj = {
    ...options.package,
    main: options.package.main ?? `cjs/${entryPointPath}`,
    module: options.package.module ?? `mjs/${entryPointPath}`,
    exports: {
      ...(options.package.exports ?? {}),
      ".": {
        "import": `./mjs/${entryPointPath}`,
        "require": `./cjs/${entryPointPath}`,
        ...(options.package.exports?.["."] ?? {}),
      }
    }
  };
  writeFile(
    path.join(options.outDir, "package.json"),
    JSON.stringify(packageJsonObj, undefined, 2),
  );

  return {
    success: true,
    diagnostics: [],
  };
}

function emitFiles(options: {
  outDir: string;
  outputFiles: OutputFile[];
  isCjs: boolean;
  typeCheck: boolean;
  writeFile: ((filePath: string, text: string) => void);
}) {
  const project = createProjectSync({
    compilerOptions: {
      outDir: options.outDir,
      allowJs: true,
      stripInternal: true,
      esModuleInterop: options.isCjs,
      module: options.isCjs ? ts.ModuleKind.CommonJS : ts.ModuleKind.ES2015,
      target: ts.ScriptTarget.ES2015,
    },
    useInMemoryFileSystem: true,
  });

  for (const outputFile of options.outputFiles) {
    project.createSourceFile(outputFile.filePath, outputFile.fileText);
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
    path.join(options.outDir, "package.json"),
    `{\n  "type": "${options.isCjs ? "commonjs" : "module"}"\n}\n`,
  );

  return {
    success: emitResult.diagnostics.length === 0,
    diagnostics: [...emitResult.diagnostics],
  };
}