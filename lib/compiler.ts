// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { ts } from "@ts-morph/bootstrap";
import * as path from "@std/path";
import { ScriptTarget } from "./types.ts";

export function outputDiagnostics(diagnostics: readonly ts.Diagnostic[]) {
  const host: ts.FormatDiagnosticsHost = {
    getCanonicalFileName: (fileName) => path.resolve(fileName),
    getCurrentDirectory: () => Deno.cwd(),
    getNewLine: () => "\n",
  };
  const output = Deno.noColor
    ? ts.formatDiagnostics(diagnostics, host)
    : ts.formatDiagnosticsWithColorAndContext(diagnostics, host);
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
    case "ES2022":
      return ts.ScriptTarget.ES2022;
    case "Latest":
      return ts.ScriptTarget.Latest;
    default:
      throw new Error(`Unknown target compiler option: ${target}`);
  }
}

// Created from https://github.com/microsoft/TypeScript/blob/v5.2.2/src/compiler/commandLineParser.ts
// then aligned with tsconfig.json's casing
export type LibName =
  | "ES5"
  | "ES6"
  | "ES2015"
  | "ES7"
  | "ES2016"
  | "ES2017"
  | "ES2018"
  | "ES2019"
  | "ES2020"
  | "ES2021"
  | "ES2022"
  | "ES2023"
  | "ESNext"
  | "DOM"
  | "DOM.Iterable"
  | "WebWorker"
  | "WebWorker.ImportScripts"
  | "WebWorker.Iterable"
  | "ScriptHost"
  | "ES2015.Core"
  | "ES2015.Collection"
  | "ES2015.Generator"
  | "ES2015.Iterable"
  | "ES2015.Promise"
  | "ES2015.Proxy"
  | "ES2015.Reflect"
  | "ES2015.Symbol"
  | "ES2015.Symbol.WellKnown"
  | "ES2016.Array.Include"
  | "ES2017.Date"
  | "ES2017.Object"
  | "ES2017.SharedMemory"
  | "ES2017.String"
  | "ES2017.Intl"
  | "ES2017.TypedArrays"
  | "ES2018.AsyncGenerator"
  | "ES2018.AsyncIterable"
  | "ES2018.Intl"
  | "ES2018.Promise"
  | "ES2018.RegExp"
  | "ES2019.Array"
  | "ES2019.Object"
  | "ES2019.String"
  | "ES2019.Symbol"
  | "ES2019.Intl"
  | "ES2020.Bigint"
  | "ES2020.Date"
  | "ES2020.Promise"
  | "ES2020.SharedMemory"
  | "ES2020.String"
  | "ES2020.Symbol.WellKnown"
  | "ES2020.Intl"
  | "ES2020.Number"
  | "ES2021.Promise"
  | "ES2021.String"
  | "ES2021.WeakRef"
  | "ES2021.Intl"
  | "ES2022.Array"
  | "ES2022.Error"
  | "ES2022.Intl"
  | "ES2022.Object"
  | "ES2022.SharedMemory"
  | "ES2022.String"
  | "ES2022.RegExp"
  | "ES2023.Array"
  | "ES2023.Collection"
  | "ESNext.Array"
  | "ESNext.Collection"
  | "ESNext.Symbol"
  | "ESNext.AsyncIterable"
  | "ESNext.Intl"
  | "ESNext.Disposable"
  | "ESNext.BigInt"
  | "ESNext.String"
  | "ESNext.Promise"
  | "ESNext.WeakRef"
  | "ESNext.Decorators"
  | "Decorators"
  | "Decorators.Legacy";

export function getCompilerLibOption(target: ScriptTarget): LibName[] {
  switch (target) {
    case "ES3":
      return [];
    case "ES5":
      return ["ES5"];
    case "ES2015":
      return ["ES2015"];
    case "ES2016":
      return ["ES2016"];
    case "ES2017":
      return ["ES2017"];
    case "ES2018":
      return ["ES2018"];
    case "ES2019":
      return ["ES2019"];
    case "ES2020":
      return ["ES2020"];
    case "ES2021":
      return ["ES2021"];
    case "ES2022":
      return ["ES2022"];
    case "Latest":
      return ["ESNext"];
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
    const fileName = libMap.get(name.toLowerCase());
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
