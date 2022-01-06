// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { OutputFile } from "../transform.ts";
import { SourceMapOptions } from "./compiler.ts";

export function getNpmIgnoreText(options: {
  sourceMap?: SourceMapOptions;
  inlineSources?: boolean;
  testFiles: OutputFile[];
}) {
  // Try to make as little of this conditional in case a user edits settings
  // to exclude something, but then the output directory still has that file
  const lines = [];
  if (!isUsingSourceMaps() || options.inlineSources) {
    lines.push("src/");
  }
  for (const fileName of getTestFileNames()) {
    lines.push(fileName);
  }
  lines.push("yarn.lock", "pnpm-lock.yaml");
  return Array.from(lines).join("\n") + "\n";

  function* getTestFileNames() {
    for (const file of options.testFiles) {
      // ignore test declaration files as they won't show up in the emit
      if (/\.d\.ts$/i.test(file.filePath)) {
        continue;
      }

      const filePath = file.filePath.replace(/\.ts$/i, ".js");
      yield `esm/${filePath}`;
      yield `umd/${filePath}`;
    }
    yield "test_runner.js";
  }

  function isUsingSourceMaps() {
    return options?.sourceMap === "inline" ||
      options?.sourceMap === true;
  }
}
