// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { path, ts } from "./mod.deps.ts";
import { ScriptTarget } from "./types.ts";

export function outputDiagnostics(diagnostics: readonly ts.Diagnostic[]) {
  const host: ts.FormatDiagnosticsHost = {
    getCanonicalFileName: (fileName) => path.resolve(fileName),
    getCurrentDirectory: () => Deno.cwd(),
    getNewLine: () => "\n",
  };
  const output = Deno.env.get("NO_COLOR") == null
    ? ts.formatDiagnosticsWithColorAndContext(diagnostics, host)
    : ts.formatDiagnostics(diagnostics, host);
  console.error(output);
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

// Created from https://github.com/microsoft/TypeScript/blob/0ad5f82d6243db80d42bc0abb7a191dd380e980e/src/compiler/commandLineParser.ts
export type LibName =
  | "es5"
  | "es6"
  | "es2015"
  | "es7"
  | "es2016"
  | "es2017"
  | "es2018"
  | "es2019"
  | "es2020"
  | "es2021"
  | "es2022"
  | "esnext"
  | "dom"
  | "dom.iterable"
  | "webworker"
  | "webworker.importscripts"
  | "webworker.iterable"
  | "scripthost"
  | "es2015.core"
  | "es2015.collection"
  | "es2015.generator"
  | "es2015.iterable"
  | "es2015.promise"
  | "es2015.proxy"
  | "es2015.reflect"
  | "es2015.symbol"
  | "es2015.symbol.wellknown"
  | "es2016.array.include"
  | "es2017.object"
  | "es2017.sharedmemory"
  | "es2017.string"
  | "es2017.intl"
  | "es2017.typedarrays"
  | "es2018.asyncgenerator"
  | "es2018.asynciterable"
  | "es2018.intl"
  | "es2018.promise"
  | "es2018.regexp"
  | "es2019.array"
  | "es2019.object"
  | "es2019.string"
  | "es2019.symbol"
  | "es2020.bigint"
  | "es2020.date"
  | "es2020.promise"
  | "es2020.sharedmemory"
  | "es2020.string"
  | "es2020.symbol.wellknown"
  | "es2020.intl"
  | "es2020.number"
  | "es2021.promise"
  | "es2021.string"
  | "es2021.weakref"
  | "es2021.intl"
  | "es2022.array"
  | "es2022.error"
  | "es2022.intl"
  | "es2022.object"
  | "es2022.string"
  | "esnext.array"
  | "esnext.symbol"
  | "esnext.asynciterable"
  | "esnext.intl"
  | "esnext.bigint"
  | "esnext.string"
  | "esnext.promise"
  | "esnext.weakref";

export function getCompilerLibOption(target: ScriptTarget): LibName[] {
  switch (target) {
    case "ES3":
      return [];
    case "ES5":
      return ["es5"];
    case "ES2015":
      return ["es2015"];
    case "ES2016":
      return ["es2016"];
    case "ES2017":
      return ["es2017"];
    case "ES2018":
      return ["es2018"];
    case "ES2019":
      return ["es2019"];
    case "ES2020":
      return ["es2020"];
    case "ES2021":
      return ["es2021"];
    case "Latest":
      return ["esnext"];
    default: {
      const _assertNever: never = target;
      throw new Error(`Unknown target compiler option: ${target}`);
    }
  }
}

export function libNamesToCompilerOption(names: LibName[]) {
  const libFileNames: string[] = [];
  const libMap = (ts as any).libMap as Map<string, string>;
  for (const name of names) {
    const fileName = libMap.get(name);
    if (fileName == null) {
      throw new Error(`Could not find filename for lib: ${name}`);
    } else {
      libFileNames.push(fileName);
    }
  }
  return libFileNames;
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
