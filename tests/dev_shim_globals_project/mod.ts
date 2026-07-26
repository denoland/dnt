// Copyright 2018-2024 the Deno authors. MIT license.

/** Gets which of the provided names exist on `globalThis`.
 *
 * @remarks This module is not transformed by the shims. It relies on the
 * globals being set when running the tests.
 */
export function getExistingGlobalNames(names: string[]) {
  return names.filter((name) =>
    // dnt-shim-ignore
    name in globalThis
  );
}
