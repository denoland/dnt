// Copyright 2018-2024 the Deno authors. MIT license.
import CodeBlockWriter from "code-block-writer";

export function getResult() {
  return new CodeBlockWriter().write("test").toString();
}
