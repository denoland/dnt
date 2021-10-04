// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { createProjectSync, path, ts } from "./lib/_mod.deps.ts";
import { outputDiagnostics } from "./lib/_compiler.ts";
import { transform } from "./transform.ts";

export * from "./transform.ts";

export interface EmitOptions {
  compilerOptions: ts.CompilerOptions;
  typeCheck?: boolean;
  entryPoint: string | URL;
  shimPackageName?: string;
  writeFile?: (filePath: string, text: string) => void;
  outputDiagnostics?: (diagnostics: readonly ts.Diagnostic[]) => void;
}

export async function emit(options: EmitOptions) {
  if (!options.compilerOptions.outDir) {
    throw new Error("Please specify an outDir compiler option.");
  }

  const outputFiles = await transform({
    entryPoint: options.entryPoint,
    shimPackageName: options.shimPackageName,
    keepExtensions: shouldKeepExtensions(),
  });
  const project = createProjectSync({
    compilerOptions: options.compilerOptions,
    useInMemoryFileSystem: true,
  });

  for (const outputFile of outputFiles) {
    project.createSourceFile(outputFile.filePath, outputFile.fileText);
  }

  const program = project.createProgram();

  if (options.typeCheck) {
    const diagnostics = ts.getPreEmitDiagnostics(program);
    if (diagnostics.length > 0) {
      (options.outputDiagnostics ?? outputDiagnostics)(diagnostics);
      Deno.exit(1);
    }
  }

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
  const emitResult = program.emit(
    undefined,
    (filePath, data, writeByteOrderMark) => {
      if (writeByteOrderMark) {
        data = "\uFEFF" + data;
      }
      writeFile(filePath, data);
    },
  );

  if (emitResult.diagnostics.length > 0) {
    outputDiagnostics(emitResult.diagnostics);
    Deno.exit(1);
  }

  function shouldKeepExtensions() {
    return options.compilerOptions.module === ts.ModuleKind.ES2015 ||
      options.compilerOptions.module === ts.ModuleKind.ES2020 ||
      options.compilerOptions.module === ts.ModuleKind.ESNext;
  }
}
