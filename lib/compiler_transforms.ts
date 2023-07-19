// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { ts } from "./mod.deps.ts";

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
    // find `import.meta.resolve`
    if (
      ts.isCallExpression(node) &&
      node.arguments.length === 1 &&
      isImportMetaProp(node.expression) &&
      node.expression.name.escapedText === "resolve"
    ) {
      return ts.visitEachChild(
        getReplacementImportMetaResolve(node.arguments),
        visitNode,
        context,
      );
    } else if (isImportMetaProp(node)) {
      // find `import.meta.url` or `import.meta.main`
      if (node.name.escapedText === "url" && isScriptModule) {
        return getReplacementImportMetaUrl();
      } else if (node.name.escapedText === "main") {
        if (isScriptModule) {
          return getReplacementImportMetaMainScript();
        } else {
          return getReplacementImportMetaMainEsm();
        }
      }
    }

    return ts.visitEachChild(node, visitNode, context);
  }

  function isImportMetaProp(
    node: ts.Node,
  ): node is ts.PropertyAccessExpression & { name: ts.Identifier } {
    return ts.isPropertyAccessExpression(node) &&
      ts.isMetaProperty(node.expression) &&
      node.expression.keywordToken === ts.SyntaxKind.ImportKeyword &&
      ts.isIdentifier(node.name);
  }

  function getReplacementImportMetaUrl() {
    // Copy and pasted from ts-ast-viewer.com
    // require("url").pathToFileURL(__filename).href
    return factory.createPropertyAccessExpression(
      factory.createCallExpression(
        factory.createPropertyAccessExpression(
          factory.createCallExpression(
            factory.createIdentifier("require"),
            undefined,
            [factory.createStringLiteral("url")],
          ),
          factory.createIdentifier("pathToFileURL"),
        ),
        undefined,
        [factory.createIdentifier("__filename")],
      ),
      factory.createIdentifier("href"),
    );
  }

  function getReplacementImportMetaMainScript() {
    // Copy and pasted from ts-ast-viewer.com
    // (require.main === module)
    return factory.createParenthesizedExpression(factory.createBinaryExpression(
      factory.createPropertyAccessExpression(
        factory.createIdentifier("require"),
        factory.createIdentifier("main"),
      ),
      factory.createToken(ts.SyntaxKind.EqualsEqualsEqualsToken),
      factory.createIdentifier("module"),
    ));
  }

  function getReplacementImportMetaMainEsm() {
    // Copy and pasted from ts-ast-viewer.com
    // (import.meta.url === ('file:///'+process.argv[1].replace(/\\/g,'/')).replace(/\/{3,}/,'///'));
    // 1. `process.argv[1]` is fullpath;
    // 2. Win's path is `E:\path\to\main.mjs`, replace to `E:/path/to/main.mjs`
    return factory.createParenthesizedExpression(
      factory.createBinaryExpression(
        factory.createPropertyAccessExpression(
          factory.createMetaProperty(
            ts.SyntaxKind.ImportKeyword,
            factory.createIdentifier("meta"),
          ),
          factory.createIdentifier("url"),
        ),
        factory.createToken(ts.SyntaxKind.EqualsEqualsEqualsToken),
        factory.createCallExpression(
          factory.createPropertyAccessExpression(
            factory.createParenthesizedExpression(
              factory.createBinaryExpression(
                factory.createStringLiteral("file:///"),
                factory.createToken(ts.SyntaxKind.PlusToken),
                factory.createCallExpression(
                  factory.createPropertyAccessExpression(
                    factory.createElementAccessExpression(
                      factory.createPropertyAccessExpression(
                        factory.createIdentifier("process"),
                        factory.createIdentifier("argv"),
                      ),
                      factory.createNumericLiteral("1"),
                    ),
                    factory.createIdentifier("replace"),
                  ),
                  undefined,
                  [
                    factory.createRegularExpressionLiteral("/\\\\/g"),
                    factory.createStringLiteral("/"),
                  ],
                ),
              ),
            ),
            factory.createIdentifier("replace"),
          ),
          undefined,
          [
            factory.createRegularExpressionLiteral("/\\/{3,}/"),
            factory.createStringLiteral("///"),
          ],
        ),
      ),
    );
  }

  function getReplacementImportMetaResolve(args: ts.NodeArray<ts.Expression>) {
    // Copy and pasted from ts-ast-viewer.com
    // new URL(specifier, import.meta.url).href
    return factory.createPropertyAccessExpression(
      factory.createNewExpression(
        factory.createIdentifier("URL"),
        undefined,
        [
          ...args,
          factory.createPropertyAccessExpression(
            factory.createMetaProperty(
              ts.SyntaxKind.ImportKeyword,
              factory.createIdentifier("meta"),
            ),
            factory.createIdentifier("url"),
          ),
        ],
      ),
      factory.createIdentifier("href"),
    );
  }
};
