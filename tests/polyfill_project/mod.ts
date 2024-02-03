// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

export function hasOwn(a: { prop?: number }) {
  try {
    return Object.hasOwn(a, "prop");
  } catch (err) {
    err.cause = new Error("test");
  }
}

export function withResolvers<T>() {
  return Promise.withResolvers<T>();
}