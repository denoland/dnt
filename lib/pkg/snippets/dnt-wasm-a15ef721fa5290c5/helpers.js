import { createCache } from "https://deno.land/x/deno_cache@0.6.2/mod.ts";

const fileFetcher = createCache();

export function fetch_specifier(specifier, cacheSettingVal) {
  return fileFetcher.load(new URL(specifier), getCacheSetting(cacheSettingVal));
}

function getCacheSetting(val) {
  // WARNING: ensure this matches wasm/src/lib.rs
  switch (val) {
    case 1:
      return "use";
    case 2:
      return "reload";
    case 0:
    default:
      return "only";
  }
}
