// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { path } from "./lib/transform.deps.ts";
import init, * as wasmFuncs from "./lib/pkg/dnt_wasm.js";

await init(getWasmLoadPromise());

export interface TransformOptions {
  entryPoint: string | URL;
  shimPackageName: string;
  /** The specifier to bare specifier mappings. */
  specifierMappings?: {
    [specifier: string]: string
  };
}

export interface Dependency {
  name: string;
  version: string;
}

export interface TransformOutput {
  entryPointFilePath: string;
  /** If the shim is used in any of the output files. */
  shimUsed: boolean;
  dependencies: Dependency[];
  cjsFiles: OutputFile[];
  mjsFiles: OutputFile[];
}

export interface OutputFile {
  filePath: string;
  fileText: string;
}

/** Analyzes the provided entry point to get all the dependended on modules and
 * outputs canonical TypeScript code in memory. The output of this function
 * can then be sent to the TypeScript compiler or a bundler for further processing. */
export function transform(options: TransformOptions): Promise<TransformOutput> {
  const newOptions = {
    ...options,
  };
  if (newOptions.entryPoint instanceof URL) {
    newOptions.entryPoint = newOptions.entryPoint.toString();
  } else {
    newOptions.entryPoint = path.toFileUrl(path.resolve(newOptions.entryPoint))
      .toString();
  }
  return wasmFuncs.transform(newOptions);
}

async function getWasmLoadPromise() {
  const moduleUrl = new URL(import.meta.url);
  switch (moduleUrl.protocol) {
    case "file:":
      const root = path.dirname(path.fromFileUrl(import.meta.url));
      return Deno.readFile(path.join(root, "./lib/pkg/dnt_wasm_bg.wasm"));
    case "https:":
    case "http:":
      const wasmUrl = new URL("./lib/pkg/dnt_wasm_bg.wasm", import.meta.url);
      const wasmResponse = await fetch(wasmUrl);
      if (
        wasmResponse.headers.get("content-type")?.toLowerCase().startsWith(
          "application/wasm",
        )
      ) {
        return wasmResponse;
      } else {
        return wasmResponse.arrayBuffer();
      }
    default:
      throw new Error(`Not implemented protocol: ${moduleUrl.protocol}`);
  }
}
