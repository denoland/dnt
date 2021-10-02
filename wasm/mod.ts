import * as path from "https://deno.land/std@0.106.0/path/mod.ts";
import init, * as wasmFuncs from "./pkg/d2n_wasm.js";

const root = path.dirname(path.fromFileUrl(import.meta.url));
await init(Deno.readFile(path.join(root, "./pkg/d2n_wasm_bg.wasm")));

export interface TransformOptions {
  entryPoint: string;
  keepExtensions: boolean;
  shimPackageName?: string;
}

export interface OutputFile {
  filePath: string;
  fileText: string;
}

export function transform(options: TransformOptions): Promise<OutputFile[]> {
  return wasmFuncs.transform(options);
}
