// Copyright 2018-2024 the Deno authors. MIT license.

import { ts } from "@ts-morph/bootstrap";

// transform `import.meta.url` to a replacement that works in script modules
export const transformImportMeta: ts.TransformerFactory<ts.SourceFile> = (
  context,
) => {
  const factory = context.factory;
  const compilerModule = context.getCompilerOptions().module;
  const isScriptModule = compilerModule === ts.ModuleKind.CommonJS ||
    compilerModule === ts.ModuleKind.UMD;

  return (sourceFile) => ts.visitEachChild(sourceFile, visitNode, context);

  function visitNode(node: ts.Node): ts.Node {
    // find `import.meta`
    if (ts.isMetaProperty(node)) {
      if (isScriptModule) {
        return getReplacementImportMetaScript();
      } else {
        return getReplacementImportMetaEsm();
      }
    }

    return ts.visitEachChild(node, visitNode, context);
  }

  function getReplacementImportMeta(
    symbolFor: string,
    argumentsArray: readonly ts.Expression[],
  ) {
    // Copy and pasted from ts-ast-viewer.com
    // globalThis[Symbol.for('import-meta-ponyfill')](...args)
    return factory.createCallExpression(
      factory.createElementAccessExpression(
        factory.createIdentifier("globalThis"),
        factory.createCallExpression(
          factory.createPropertyAccessExpression(
            factory.createIdentifier("Symbol"),
            factory.createIdentifier("for"),
          ),
          undefined,
          [factory.createStringLiteral(symbolFor)],
        ),
      ),
      undefined,
      argumentsArray,
    );
  }
  function getReplacementImportMetaScript() {
    return getReplacementImportMeta("import-meta-ponyfill-commonjs", [
      factory.createIdentifier("require"),
      factory.createIdentifier("module"),
    ]);
  }
  function getReplacementImportMetaEsm() {
    return getReplacementImportMeta("import-meta-ponyfill-esmodule", [
      factory.createMetaProperty(
        ts.SyntaxKind.ImportKeyword,
        factory.createIdentifier("meta"),
      ),
    ]);
  }
};
