// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { path, ts } from "./mod.deps.ts";
import { ScriptTarget } from "./types.ts";

export function outputDiagnostics(diagnostics: readonly ts.Diagnostic[]) {
  console.error(ts.formatDiagnosticsWithColorAndContext(diagnostics, {
    getCanonicalFileName: (fileName) => path.resolve(fileName),
    getCurrentDirectory: () => Deno.cwd(),
    getNewLine: () => "\n",
  }));
}

export function getCompilerScriptTarget(target: ScriptTarget) {
  switch (target) {
    case "ES3":
      return ts.ScriptTarget.ES3;
    case "ES5":
      return ts.ScriptTarget.ES5;
    case "ES2015":
      return ts.ScriptTarget.ES2015;
    case "ES2016":
      return ts.ScriptTarget.ES2016;
    case "ES2017":
      return ts.ScriptTarget.ES2017;
    case "ES2018":
      return ts.ScriptTarget.ES2018;
    case "ES2019":
      return ts.ScriptTarget.ES2019;
    case "ES2020":
      return ts.ScriptTarget.ES2020;
    case "ES2021":
      return ts.ScriptTarget.ES2021;
    case "Latest":
      return ts.ScriptTarget.Latest;
    default:
      throw new Error(`Unknown target compiler option: ${target}`);
  }
}

export type SourceMapOptions = "inline" | boolean;

export function getCompilerSourceMapOptions(
  sourceMaps: SourceMapOptions | undefined,
): { inlineSourceMap?: boolean; sourceMap?: boolean } {
  switch (sourceMaps) {
    case "inline":
      return { inlineSourceMap: true };
    case true:
      return { sourceMap: true };
    default:
      return {};
  }
}

export function getTopLevelAwait(sourceFile: ts.SourceFile) {
  for (const statement of sourceFile.statements) {
    if (
      ts.isExpressionStatement(statement) &&
      ts.isAwaitExpression(statement.expression)
    ) {
      return sourceFile.getLineAndCharacterOfPosition(
        statement.expression.getStart(sourceFile),
      );
    }
  }
  return undefined;
}

export function transformCodeToTarget(code: string, target: ts.ScriptTarget) {
  return ts.transpile(code, {
    target,
  });
}
