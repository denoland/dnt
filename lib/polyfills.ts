// Copyright 2018-2024 the Deno authors. MIT license.

import type { PolyfillName, PolyfillOptions, ScriptTarget } from "./types.ts";

/** Every polyfill dnt knows how to apply. */
export const polyfillNames: readonly PolyfillName[] = [
  "arrayFindLast",
  "arrayFromAsync",
  "errorCause",
  "importMeta",
  "objectHasOwn",
  "promiseWithResolvers",
  "stringReplaceAll",
];

/** Resolves the user provided polyfill options into an explicit
 * enabled/disabled value per polyfill.
 *
 * Polyfills that the user didn't specify are left out of the result so that
 * the script target continues to decide whether they're used.
 */
export function resolvePolyfillOptions(
  options: PolyfillOptions | undefined,
): Record<string, boolean> {
  if (options == null) {
    return {};
  }
  if (typeof options === "boolean") {
    return Object.fromEntries(polyfillNames.map((name) => [name, options]));
  }

  const resolved: Record<string, boolean> = {};
  for (const [name, enabled] of Object.entries(options)) {
    if (enabled == null) {
      continue;
    }
    if (!polyfillNames.includes(name as PolyfillName)) {
      throw new Error(
        `Unknown polyfill '${name}' specified in the 'polyfills' option. ` +
          `Supported polyfills: ${polyfillNames.join(", ")}`,
      );
    }
    resolved[name] = enabled;
  }
  return resolved;
}

/** Whether the `import.meta` polyfill applies, given the resolved polyfill
 * overrides and the rest of the build options.
 *
 * `import.meta` is a syntax error in CommonJS, so the polyfill is required
 * whenever a script module is emitted regardless of the target.
 */
export function resolveUseImportMetaPolyfill(options: {
  polyfills: Record<string, boolean>;
  target: ScriptTarget;
  emitScriptModule: boolean;
}): boolean {
  const explicit = options.polyfills["importMeta"];
  if (options.emitScriptModule) {
    if (explicit === false) {
      throw new Error(
        "The 'importMeta' polyfill cannot be disabled when emitting a script " +
          "module because `import.meta` is not valid CommonJS. Set the " +
          "'scriptModule' build option to false to distribute an ES module only.",
      );
    }
    return true;
  }
  return explicit ?? options.target !== "Latest";
}
