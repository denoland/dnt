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

export function getCompilerLibOption(target: ScriptTarget): string[] {
  switch (target) {
    case "ES3":
      return [];
    case "ES5":
      return ["lib.es5.d.ts"];
    case "ES2015":
      return ["lib.es2015.d.ts"];
    case "ES2016":
      return ["lib.es2016.d.ts"];
    case "ES2017":
      return ["lib.es2017.d.ts"];
    case "ES2018":
      return ["lib.es2018.d.ts"];
    case "ES2019":
      return ["lib.es2019.d.ts"];
    case "ES2020":
      return ["lib.es2020.d.ts"];
    case "ES2021":
      return ["lib.es2021.d.ts"];
    case "Latest":
      return ["lib.esnext.d.ts"];
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

export function getTopLevelAwaitLocation(sourceFile: ts.SourceFile) {
  const topLevelAwait = getTopLevelAwait(sourceFile);
  if (topLevelAwait !== undefined) {
    return sourceFile.getLineAndCharacterOfPosition(
      topLevelAwait.getStart(sourceFile),
    );
  }
  return undefined;
}

function getTopLevelAwait(node: ts.Node): ts.Node | undefined {
  if (ts.isAwaitExpression(node)) {
    return node;
  }
  if (ts.isForOfStatement(node) && node.awaitModifier !== undefined) {
    return node;
  }
  return ts.forEachChild(node, (child) => {
    if (
      !ts.isFunctionDeclaration(child) && !ts.isFunctionExpression(child) &&
      !ts.isArrowFunction(child) && !ts.isMethodDeclaration(child)
    ) {
      return getTopLevelAwait(child);
    }
  });
}

export function transformCodeToTarget(code: string, target: ts.ScriptTarget) {
  return ts.transpile(code, {
    target,
  });
}
