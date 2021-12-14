// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

export function hasOwn(a: { prop?: number }) {
  return Object.hasOwn(a, "prop");
}
