import * as path from "https://deno.land/std@0.109.0/path/mod.ts";

const rootDir = path.dirname(path.fromFileUrl(import.meta.url));
await buildWasm();

async function buildWasm() {
  const cmd = Deno.run({
    cmd: ["wasm-pack", "build", "--out-dir", "../deno/pkg", "--target", "web"],
    cwd: path.join(rootDir, "wasm"),
    stderr: "inherit",
    stdout: "inherit",
  })
  try {
    const status = await cmd.status();
    if (!status.success) {
      throw new Error(`Error running wasm-pack.`);
    }
  } finally {
    cmd.close();
  }

  Deno.remove(path.join(rootDir, "./deno/pkg/.gitignore"));
  Deno.remove(path.join(rootDir, "./deno/pkg/dnt_wasm_bg.wasm.d.ts"));
  Deno.remove(path.join(rootDir, "./deno/pkg/package.json"));
}
