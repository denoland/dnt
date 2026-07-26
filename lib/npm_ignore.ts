// Copyright 2018-2024 the Deno authors. MIT license.

import type { OutputFile } from "../transform.ts";
import type { SourceMapOptions } from "./compiler.ts";
import { toDtsFilePath, toJsFilePath } from "./utils.ts";

export function getNpmIgnoreText(options: {
  sourceMap?: SourceMapOptions;
  inlineSources?: boolean;
  testFiles: OutputFile[];
  declaration: "separate" | "inline" | false;
  declarationMap: boolean | undefined;
  includeScriptModule: boolean | undefined;
  includeEsModule: boolean | undefined;
}) {
  // Try to make as little of this conditional in case a user edits settings
  // to exclude something, but then the output directory still has that file
  const lines = [];
  if (!isReferencingSrcDir()) {
    lines.push("/src/");
  }
  for (const fileName of getTestFileNames()) {
    lines.push(fileName);
  }
  lines.push("yarn.lock", "pnpm-lock.yaml");
  return Array.from(lines).join("\n") + "\n";

  function* getTestFileNames() {
    for (const file of options.testFiles) {
      const filePath = toJsFilePath(file.filePath);
      const dtsFilePath = toDtsFilePath(file.filePath);
      // the whole directory is excluded above when it's not published
      if (isReferencingSrcDir()) {
        yield `/src/${file.filePath}`;
      }
      if (options.includeEsModule) {
        const esmFilePath = `/esm/${filePath}`;
        yield esmFilePath;
        if (options.sourceMap === true) {
          yield `${esmFilePath}.map`;
        }
        if (options.declaration === "inline") {
          yield `/esm/${dtsFilePath}`;
          if (options.declarationMap) {
            yield `/esm/${dtsFilePath}.map`;
          }
        }
      }
      if (options.includeScriptModule) {
        const scriptFilePath = `/script/${filePath}`;
        yield scriptFilePath;
        if (options.sourceMap === true) {
          yield `${scriptFilePath}.map`;
        }
        if (options.declaration === "inline") {
          yield `/script/${dtsFilePath}`;
          if (options.declarationMap) {
            yield `/script/${dtsFilePath}.map`;
          }
        }
      }
      if (options.declaration === "separate") {
        yield `/types/${dtsFilePath}`;
        if (options.declarationMap) {
          yield `/types/${dtsFilePath}.map`;
        }
      }
    }
    yield "/test_runner.cjs";
  }

  /** Whether any emitted map points back at the files in `/src/`, in which
   * case the directory needs to be published for the map to resolve. */
  function isReferencingSrcDir() {
    // `inlineSources` embeds the sources in the source map, so `/src/` is only
    // needed without it. It has no effect on declaration maps though, so those
    // always need the directory.
    return (isUsingSourceMaps() && !options.inlineSources) ||
      isEmittingDeclarationMaps();
  }

  function isEmittingDeclarationMaps() {
    return options.declaration !== false && !!options.declarationMap;
  }

  function isUsingSourceMaps() {
    return options?.sourceMap === "inline" ||
      options?.sourceMap === true;
  }
}
