import { path, ts } from "./mod.deps.ts";

export function outputDiagnostics(diagnostics: readonly ts.Diagnostic[]) {
  console.error(ts.formatDiagnosticsWithColorAndContext(diagnostics, {
    getCanonicalFileName: (fileName) => path.resolve(fileName),
    getCurrentDirectory: () => Deno.cwd(),
    getNewLine: () => "\n",
  }));
}

export class DiagnosticsError extends Error {
  constructor(public readonly diagnostics: readonly ts.Diagnostic[]) {
    super(diagnostics[0]?.messageText?.toString() ?? "Unknown error.");
  }
}
