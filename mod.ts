// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { createProjectSync, path, ts } from "./lib/mod.deps.ts";
import { transform } from "./transform.ts";

export * from "./transform.ts";

// necessary for use with compiler options
export { ts };

export interface EmitOptions {
  compilerOptions: ts.CompilerOptions;
  typeCheck?: boolean;
  entryPoint: string | URL;
  shimPackageName?: string;
  writeFile?: (filePath: string, text: string) => void;
}

export interface EmitResult {
  success: boolean;
  diagnostics: ts.Diagnostic[];
}

/** Emits the specified Deno module to JavaScript code using the TypeScript compiler. */
export async function emit(options: EmitOptions): Promise<EmitResult> {
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
      return {
        success: false,
        diagnostics: [...diagnostics],
      };
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
    return {
      success: false,
      diagnostics: [...emitResult.diagnostics],
    };
  }

  return {
    success: true,
    diagnostics: [],
  };

  function shouldKeepExtensions() {
    return options.compilerOptions.module === ts.ModuleKind.ES2015 ||
      options.compilerOptions.module === ts.ModuleKind.ES2020 ||
      options.compilerOptions.module === ts.ModuleKind.ESNext;
  }
}
