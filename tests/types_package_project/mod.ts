// Copyright 2018-2024 the Deno authors. MIT license.

import type { Node } from "unist";

export function getType(node: Node) {
  return node.type;
}
