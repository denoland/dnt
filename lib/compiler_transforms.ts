// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { ts } from "./mod.deps.ts";

// transform `import.meta.url` to a replacement that works in script modules
export const transformImportMeta: ts.TransformerFactory<ts.SourceFile> = (
  context,
) => {
  const factory = context.factory;

  return (sourceFile) => ts.visitEachChild(sourceFile, visitNode, context);

  function visitNode(node: ts.Node): ts.Node {
    // find `import.meta.url`
    if (
      ts.isPropertyAccessExpression(node) &&
      ts.isMetaProperty(node.expression) &&
      node.expression.keywordToken === ts.SyntaxKind.ImportKeyword &&
      ts.isIdentifier(node.name)
    ) {
      if (node.name.escapedText === "url") {
        return getReplacementImportMetaUrl();
      } else if (node.name.escapedText === "main") {
        return getReplacementImportMetaMain();
      }
    }

    return ts.visitEachChild(node, visitNode, context);
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

  function getReplacementImportMetaMain() {
    // Copy and pasted from ts-ast-viewer.com
    // (require.main === module)
    return factory.createParenthesizedExpression(factory.createBinaryExpression(
      factory.createPropertyAccessExpression(
        factory.createIdentifier("require"),
        factory.createIdentifier("main")
      ),
      factory.createToken(ts.SyntaxKind.EqualsEqualsEqualsToken),
      factory.createIdentifier("module")
    ))
  }
};
