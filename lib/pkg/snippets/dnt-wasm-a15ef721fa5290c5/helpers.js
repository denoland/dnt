import { createCache } from "@deno/cache-dir";

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
