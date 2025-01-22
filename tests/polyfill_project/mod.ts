// Copyright 2018-2024 the Deno authors. MIT license.

export function hasOwn(a: { prop?: number }) {
  try {
    return Object.hasOwn(a, "prop");
  } catch (err) {
    (err as any).cause = new Error("test");
  }
}

export function withResolvers<T>() {
  return Promise.withResolvers<T>();
}
