// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import * as path from "https://deno.land/std@0.140.0/path/mod.ts";
import { encode } from "https://deno.land/std@0.140.0/encoding/base64.ts";

const rootDir = path.dirname(path.fromFileUrl(import.meta.url));
await buildWasm();

async function buildWasm() {
  const cmd = Deno.run({
    cmd: ["wasm-pack", "build", "--out-dir", "../lib/pkg", "--target", "web"],
    cwd: path.join(rootDir, "wasm"),
    stderr: "inherit",
    stdout: "inherit",
  });
  try {
    const status = await cmd.status();
    if (!status.success) {
      throw new Error(`Error running wasm-pack.`);
    }
  } finally {
    cmd.close();
  }

  const wasmFilePath = path.join(rootDir, "./lib/pkg/dnt_wasm_bg.wasm");
  const wasmBytes = Deno.readFileSync(wasmFilePath);

  await Deno.writeTextFile(
    path.join(rootDir, "./lib/pkg/dnt_wasm_bg.ts"),
    `export const source = Uint8Array.from(atob("${
      encode(wasmBytes)
    }"), c => c.charCodeAt(0));\n`,
  );
  await Deno.remove(wasmFilePath);
  await Deno.remove(path.join(rootDir, "./lib/pkg/.gitignore"));
  await Deno.remove(path.join(rootDir, "./lib/pkg/dnt_wasm_bg.wasm.d.ts"));
  await Deno.remove(path.join(rootDir, "./lib/pkg/package.json"));
}
