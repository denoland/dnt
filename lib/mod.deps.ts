// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

export * from "./transform.deps.ts";
export {
  createProjectSync,
  ts,
} from "https://deno.land/x/ts_morph@12.0.0/bootstrap/mod.ts";
export * as colors from "https://deno.land/std@0.111.0/fmt/colors.ts";
