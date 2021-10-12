import { DenoDir, FileFetcher } from "https://deno.land/x/deno_cache@0.1.0/mod.ts";

const denoDir = new DenoDir();
const fileFetcher = new FileFetcher(denoDir.deps);

export function fetch_specifier(specifier) {
  return fileFetcher.fetch(new URL(specifier));
}
