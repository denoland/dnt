// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
import CodeBlockWriter from "https://deno.land/x/code_block_writer@11.0.0/mod.ts";
import { using } from "npm:using-statement@^0.4";

export function getResult() {
  console.log(using);
  return new CodeBlockWriter().write("test").toString();
}
