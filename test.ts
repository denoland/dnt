import { createProject, ts } from "https://deno.land/x/ts_morph@12.0.0/bootstrap/mod.ts";
import { transform } from "./wasm/mod.ts";

const outputFiles = await transform({
  entryPoint: "file:///V:/code-block-writer/mod.ts",
  keepExtensions: false,
});

const project = await createProject({
  compilerOptions: {
    target: ts.ScriptTarget.ES2015,
    declaration: true,
    module: ts.ModuleKind.CommonJS,
    outDir: "./dist",
  }
});

for (const outputFile of outputFiles) {
  project.createSourceFile(outputFile.filePath, outputFile.fileText);
}

project.createProgram().emit();
