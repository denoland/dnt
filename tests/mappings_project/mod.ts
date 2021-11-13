// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import CodeBlockWriter from "https://deno.land/x/code_block_writer@11.0.0/mod.ts";

export function getResult() {
  return new CodeBlockWriter().write("test").toString();
}
