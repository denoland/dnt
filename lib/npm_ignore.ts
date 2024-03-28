// Copyright 2018-2024 the Deno authors. MIT license.

import type { OutputFile } from "../transform.ts";
import type { SourceMapOptions } from "./compiler.ts";

export function getNpmIgnoreText(options: {
  sourceMap?: SourceMapOptions;
  inlineSources?: boolean;
  testFiles: OutputFile[];
  declaration: "separate" | "inline" | false;
  includeScriptModule: boolean | undefined;
  includeEsModule: boolean | undefined;
}) {
  // Try to make as little of this conditional in case a user edits settings
  // to exclude something, but then the output directory still has that file
  const lines = [];
  if (!isUsingSourceMaps() || options.inlineSources) {
    lines.push("/src/");
  }
  for (const fileName of getTestFileNames()) {
    lines.push(fileName);
  }
  lines.push("yarn.lock", "pnpm-lock.yaml");
  return Array.from(lines).join("\n") + "\n";

  function* getTestFileNames() {
    for (const file of options.testFiles) {
      const filePath = file.filePath.replace(/\.ts$/i, ".js");
      const dtsFilePath = file.filePath.replace(/\.ts$/i, ".d.ts");
      if (options.includeEsModule) {
        const esmFilePath = `/esm/${filePath}`;
        yield esmFilePath;
        if (options.sourceMap === true) {
          yield `${esmFilePath}.map`;
        }
        if (options.declaration === "inline") {
          yield `/esm/${dtsFilePath}`;
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
        }
      }
      if (options.declaration === "separate") {
        yield `/types/${dtsFilePath}`;
      }
    }
    yield "/test_runner.js";
  }

  function isUsingSourceMaps() {
    return options?.sourceMap === "inline" ||
      options?.sourceMap === true;
  }
}
