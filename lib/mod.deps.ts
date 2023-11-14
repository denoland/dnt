// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

export * from "./transform.deps.ts";
export {
  createProjectSync,
  ts,
} from "https://deno.land/x/ts_morph@20.0.0/bootstrap/mod.ts";
export { default as CodeBlockWriter } from "https://deno.land/x/code_block_writer@12.0.0/mod.ts";
export * as colors from "https://deno.land/std@0.181.0/fmt/colors.ts";
export * as glob from "https://deno.land/std@0.181.0/fs/expand_glob.ts";
export { emptyDir } from "https://deno.land/std@0.181.0/fs/empty_dir.ts";
