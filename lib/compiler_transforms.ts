import { ts } from "./mod.deps.ts";

// transform `import.meta.url` to a replacement that works in UMD modules
export const transformImportMeta: ts.TransformerFactory<ts.SourceFile> = (
  context,
) => {
  return (sourceFile) => ts.visitEachChild(sourceFile, visitNode, context);

  function visitNode(node: ts.Node): ts.Node {
    // find `import.meta.url`
    if (
      ts.isPropertyAccessExpression(node) &&
      ts.isMetaProperty(node.expression) &&
      node.expression.keywordToken === ts.SyntaxKind.ImportKeyword &&
      ts.isIdentifier(node.name) &&
      node.name.escapedText === "url"
    ) {
      return getReplacementBinaryExpr();
    }
    return ts.visitEachChild(node, visitNode, context);
  }

  function getReplacementBinaryExpr(): ts.PropertyAccessExpression {
    const factory = context.factory;
    // Copy and pasted from ts-ast-viewer.com
    // require("url").pathToFileURL(__filename).href
    return factory.createPropertyAccessExpression(
      factory.createCallExpression(
        factory.createPropertyAccessExpression(
          factory.createCallExpression(
            factory.createIdentifier("require"),
            undefined,
            [factory.createStringLiteral("url")]
          ),
          factory.createIdentifier("pathToFileURL")
        ),
        undefined,
        [factory.createIdentifier("__filename")]
      ),
      factory.createIdentifier("href")
    );
  }
};
