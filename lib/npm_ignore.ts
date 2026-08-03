// Copyright 2018-2024 the Deno authors. MIT license.

import type { OutputFile } from "../transform.ts";
import type { SourceMapOptions } from "./compiler.ts";
import { isDeclarationFilePath, toDtsFilePath, toJsFilePath } from "./utils.ts";

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
      // the whole directory is excluded above when it's not published
      if (isReferencingSrcDir()) {
        yield `/src/${file.filePath}`;
      }
      // A dependency can ship a declaration file directly (ex. a prebuilt
      // package published to JSR). The compiler copies it as-is beside the
      // emitted code rather than turning it into a `.js` + `.d.ts` pair, so it
      // has different output paths than a regular source file.
      if (isDeclarationFilePath(file.filePath)) {
        yield* getDeclarationTestFileNames(file.filePath);
        continue;
      }
      const jsFilePath = toJsFilePath(file.filePath);
      const dtsFilePath = toDtsFilePath(file.filePath);
      if (options.includeEsModule) {
        const esmFilePath = `/esm/${jsFilePath}`;
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
        const scriptFilePath = `/script/${jsFilePath}`;
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

  /** A declaration file emits no `.js`, so the compiler only copies it where
   * declarations are output: beside the code when they're inlined, or the
   * `types` directory when they're kept separate. */
  function* getDeclarationTestFileNames(dtsFilePath: string) {
    if (options.declaration === "inline") {
      if (options.includeEsModule) {
        yield `/esm/${dtsFilePath}`;
        if (options.declarationMap) {
          yield `/esm/${dtsFilePath}.map`;
        }
      }
      if (options.includeScriptModule) {
        yield `/script/${dtsFilePath}`;
        if (options.declarationMap) {
          yield `/script/${dtsFilePath}.map`;
        }
      }
    } else if (options.declaration === "separate") {
      yield `/types/${dtsFilePath}`;
      if (options.declarationMap) {
        yield `/types/${dtsFilePath}.map`;
      }
    }
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
