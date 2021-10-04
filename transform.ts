// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { path } from "./lib/_transform.deps.ts";
import init, * as wasmFuncs from "./lib/pkg/dnt_wasm.js";

await init(getWasmLoadPromise());

export interface TransformOptions {
  entryPoint: string | URL;
  keepExtensions: boolean;
  shimPackageName?: string;
}

export interface OutputFile {
  filePath: string;
  fileText: string;
}

export function transform(options: TransformOptions): Promise<OutputFile[]> {
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
