// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

export function hasOwn(a: { prop?: number }) {
  try {
    return Object.hasOwn(a, "prop");
  } catch (err) {
    err.cause = new Error("test");
  }
}
