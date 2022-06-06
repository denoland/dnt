import type { MyType } from "./types.d.ts";
import type {
  RawSourceMap,
  SourceMapUrl,
} from "https://esm.sh/source-map@0.7.3/source-map.d.ts";

export function main(): MyType {
  return { prop: "" };
}

export function other(): RawSourceMap {
  const _test: SourceMapUrl = "";
  return {
    file: "",
    mappings: "",
    names: [],
    sources: [],
    version: 2,
  };
}