import { createCache } from "https://deno.land/x/deno_cache@0.5.2/mod.ts";

const fileFetcher = createCache();

export function fetch_specifier(specifier) {
  return fileFetcher.load(new URL(specifier));
}
