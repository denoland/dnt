// Copyright 2018-2024 the Deno authors. MIT license.

/** Gets which of the provided names exist on `globalThis`.
 *
 * @remarks This module is distributed to npm as-is. The globals it uses are
 * expected to exist, which the test preload module ensures when testing.
 */
export function getExistingGlobalNames(names: string[]) {
  return names.filter((name) =>
    // dnt-shim-ignore
    name in globalThis
  );
}
