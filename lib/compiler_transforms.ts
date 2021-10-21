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

  function getReplacementBinaryExpr(): ts.BinaryExpression {
    const factory = context.factory;
    // Copy and pasted from ts-ast-viewer.com
    // document.currentScript && document.currentScript.src || document.baseURI
    return factory.createBinaryExpression(
      factory.createBinaryExpression(
        factory.createPropertyAccessExpression(
          factory.createIdentifier("document"),
          factory.createIdentifier("currentScript"),
        ),
        factory.createToken(ts.SyntaxKind.AmpersandAmpersandToken),
        factory.createPropertyAccessExpression(
          factory.createPropertyAccessExpression(
            factory.createIdentifier("document"),
            factory.createIdentifier("currentScript"),
          ),
          factory.createIdentifier("src"),
        ),
      ),
      factory.createToken(ts.SyntaxKind.BarBarToken),
      factory.createPropertyAccessExpression(
        factory.createIdentifier("document"),
        factory.createIdentifier("baseURI"),
      ),
    );
  }
};
