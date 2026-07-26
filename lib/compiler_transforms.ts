// Copyright 2018-2024 the Deno authors. MIT license.

import { ts } from "@ts-morph/bootstrap";

// identifiers the emit references in the module scope (ex.
// `Object.defineProperty(exports, "__esModule", { value: true });` for
// CommonJS or `Object.defineProperty(this, "prop", ...)` when downleveling
// class fields)
const emitGlobalNames = new Set([
  "Array",
  "Object",
  "Promise",
  "Reflect",
  "Symbol",
  "SuppressedError",
  "TypeError",
  "exports",
  "globalThis",
  "module",
  "require",
]);

/**
 * Creates a transformer that renames module scoped declarations shadowing an
 * identifier the emit relies on.
 *
 * For example, a module declaring `const Object = "hello";` would otherwise
 * emit CommonJS code that throws at runtime because the emitted
 * `Object.defineProperty(exports, "__esModule", { value: true });` would
 * resolve `Object` to the module's declaration instead of the global.
 */
export function createShadowedGlobalsTransformer(
  program: ts.Program,
): ts.TransformerFactory<ts.SourceFile> {
  return (context) => {
    const factory = context.factory;
    const compilerModule = context.getCompilerOptions().module;
    const isScriptModule = compilerModule === ts.ModuleKind.CommonJS ||
      compilerModule === ts.ModuleKind.UMD;

    return (sourceFile) => {
      if (sourceFile.isDeclarationFile) {
        return sourceFile;
      }
      const declarationNames = Array.from(
        getShadowingDeclarationNames(sourceFile, isScriptModule),
      );
      if (declarationNames.length === 0) {
        return sourceFile; // fast path—only get the type checker when necessary
      }
      const checker = program.getTypeChecker();
      const renames = getRenames();
      if (renames.size === 0) {
        return sourceFile;
      }
      // exports of renamed declarations are re-added at the end of the file
      // in order to keep the original export names (ex. a renamed
      // `export class Object {}` still needs to be exported as `Object`)
      const appendedExports: ExportName[] = [];
      const statements = sourceFile.statements.map((statement) => {
        const exportNames = getExportNamesToAppend(statement);
        const newStatement = visitNode(statement) as ts.Statement;
        if (exportNames.length === 0) {
          return newStatement;
        }
        appendedExports.push(...exportNames);
        return removeExportModifier(newStatement);
      });
      return factory.updateSourceFile(sourceFile, [
        ...statements,
        ...createAppendedExports(),
      ]);

      function visitNode(node: ts.Node): ts.Node | undefined {
        // `export { Object as name };` -> `export { Object_1 as name };`
        if (ts.isExportSpecifier(node)) {
          const localName = getRenamedName(
            checker.getExportSpecifierLocalTargetSymbol(node),
          );
          if (localName != null) {
            if (isScriptModule) {
              // the CommonJS emit can't resolve a renamed export specifier,
              // so export it at the end of the file instead
              appendedExports.push({ localName, exportName: node.name });
              return undefined;
            }
            return factory.updateExportSpecifier(
              node,
              node.isTypeOnly,
              factory.createIdentifier(localName),
              node.name,
            );
          }
          return node;
        }

        // `{ Object }` -> `{ Object: Object_1 }`
        if (ts.isShorthandPropertyAssignment(node)) {
          const newName = getRenamedName(
            checker.getShorthandAssignmentValueSymbol(node),
          );
          if (newName != null) {
            const value = factory.createIdentifier(newName);
            return factory.createPropertyAssignment(
              node.name,
              node.objectAssignmentInitializer == null ? value : factory
                .createAssignment(
                  value,
                  visitNode(node.objectAssignmentInitializer) as ts.Expression,
                ),
            );
          }
        }

        // `const { Object } = value;` -> `const { Object: Object_1 } = value;`
        if (
          ts.isBindingElement(node) && node.propertyName == null &&
          node.dotDotDotToken == null && ts.isIdentifier(node.name) &&
          ts.isObjectBindingPattern(node.parent)
        ) {
          const newName = getRenamedNameAtLocation(node.name);
          if (newName != null) {
            return factory.updateBindingElement(
              node,
              node.dotDotDotToken,
              node.name,
              factory.createIdentifier(newName),
              node.initializer == null
                ? undefined
                : visitNode(node.initializer) as ts.Expression,
            );
          }
        }

        if (ts.isIdentifier(node)) {
          const newName = getRenamedNameAtLocation(node);
          if (newName != null) {
            return factory.createIdentifier(newName);
          }
        }

        return ts.visitEachChild(node, visitNode, context);
      }

      function getRenames() {
        const usedNames = getIdentifierNames(sourceFile);
        const renames = new Map<ts.Symbol, string>();
        for (const declarationName of declarationNames) {
          const symbol = checker.getSymbolAtLocation(declarationName);
          if (symbol == null || renames.has(symbol)) {
            continue;
          }
          let index = 1;
          let newName = `${declarationName.text}_${index}`;
          while (usedNames.has(newName)) {
            newName = `${declarationName.text}_${++index}`;
          }
          usedNames.add(newName);
          renames.set(symbol, newName);
        }
        return renames;
      }

      /** Gets the export names of a statement that need to be re-added when
       * it declares a renamed name. */
      function getExportNamesToAppend(statement: ts.Statement): ExportName[] {
        if (
          !hasModifier(statement, ts.SyntaxKind.ExportKeyword) ||
          // `export default class Object {}` keeps its export name
          hasModifier(statement, ts.SyntaxKind.DefaultKeyword)
        ) {
          return [];
        }
        const names = ts.isVariableStatement(statement)
          ? statement.declarationList.declarations.flatMap((d) =>
            Array.from(getBindingNames(d.name))
          )
          : isRenameableDeclaration(statement) && statement.name != null &&
              ts.isIdentifier(statement.name)
          ? [statement.name]
          : [];
        const exportNames = names.map((name) => ({
          localName: getRenamedNameAtLocation(name) ?? name.text,
          exportName: name,
        }));
        return exportNames.some((n) => n.localName !== n.exportName.text)
          ? exportNames
          : [];
      }

      function createAppendedExports(): ts.Statement[] {
        if (appendedExports.length === 0) {
          return [];
        }
        if (!isScriptModule) {
          return [factory.createExportDeclaration(
            undefined,
            false,
            factory.createNamedExports(
              appendedExports.map(({ localName, exportName }) =>
                factory.createExportSpecifier(
                  false,
                  factory.createIdentifier(localName),
                  createExportName(exportName),
                )
              ),
            ),
          )];
        }
        return appendedExports.map(({ localName, exportName }) => {
          const localIdentifier = factory.createIdentifier(localName);
          if (!ts.isIdentifier(exportName)) {
            // ex. `exports["some name"] = Object_1;`
            return factory.createExpressionStatement(factory.createAssignment(
              factory.createElementAccessExpression(
                factory.createIdentifier("exports"),
                factory.createStringLiteral(exportName.text),
              ),
              localIdentifier,
            ));
          }
          if (exportName.text === "default") {
            return factory.createExportDefault(localIdentifier);
          }
          // an exported variable declaration is used here instead of an export
          // specifier because the CommonJS emit does not create a module scoped
          // binding for it, which would otherwise shadow the global again
          return factory.createVariableStatement(
            [factory.createModifier(ts.SyntaxKind.ExportKeyword)],
            factory.createVariableDeclarationList([
              factory.createVariableDeclaration(
                exportName.text,
                undefined,
                undefined,
                localIdentifier,
              ),
            ], ts.NodeFlags.Const),
          );
        });
      }

      function createExportName(name: ts.Identifier | ts.StringLiteral) {
        return ts.isIdentifier(name)
          ? factory.createIdentifier(name.text)
          : factory.createStringLiteral(name.text);
      }

      function removeExportModifier(statement: ts.Statement) {
        const modifiers = ts.canHaveModifiers(statement)
          ? ts.getModifiers(statement)?.filter((m) =>
            m.kind !== ts.SyntaxKind.ExportKeyword
          )
          : undefined;
        if (ts.isFunctionDeclaration(statement)) {
          return factory.updateFunctionDeclaration(
            statement,
            modifiers,
            statement.asteriskToken,
            statement.name,
            statement.typeParameters,
            statement.parameters,
            statement.type,
            statement.body,
          );
        } else if (ts.isClassDeclaration(statement)) {
          return factory.updateClassDeclaration(
            statement,
            modifiers,
            statement.name,
            statement.typeParameters,
            statement.heritageClauses,
            statement.members,
          );
        } else if (ts.isEnumDeclaration(statement)) {
          return factory.updateEnumDeclaration(
            statement,
            modifiers,
            statement.name,
            statement.members,
          );
        } else if (ts.isModuleDeclaration(statement)) {
          return factory.updateModuleDeclaration(
            statement,
            modifiers,
            statement.name,
            statement.body,
          );
        } else if (ts.isVariableStatement(statement)) {
          return factory.updateVariableStatement(
            statement,
            modifiers,
            statement.declarationList,
          );
        } else {
          return statement;
        }
      }

      function getRenamedNameAtLocation(node: ts.Identifier) {
        if (!emitGlobalNames.has(node.text)) {
          return undefined;
        }
        return getRenamedName(checker.getSymbolAtLocation(node));
      }

      function getRenamedName(symbol: ts.Symbol | undefined) {
        return symbol == null ? undefined : renames.get(symbol);
      }
    };
  };
}

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
    // find `import.meta` (not `new.target`, which is also a meta property)
    if (
      ts.isMetaProperty(node) &&
      node.keywordToken === ts.SyntaxKind.ImportKeyword
    ) {
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

interface ExportName {
  localName: string;
  exportName: ts.Identifier | ts.StringLiteral;
}

/** Gets the module scoped declaration names that shadow an identifier the
 * emit relies on. */
function* getShadowingDeclarationNames(
  sourceFile: ts.SourceFile,
  isScriptModule: boolean,
): Generator<ts.Identifier> {
  for (const statement of sourceFile.statements) {
    if (hasModifier(statement, ts.SyntaxKind.DeclareKeyword)) {
      continue; // ambient declarations don't emit
    }
    if (ts.isVariableStatement(statement)) {
      // exported variables become properties on `exports` in the CommonJS
      // emit, so they never create a module scoped binding there
      if (
        isScriptModule && hasModifier(statement, ts.SyntaxKind.ExportKeyword)
      ) {
        continue;
      }
      for (const declaration of statement.declarationList.declarations) {
        for (const name of getBindingNames(declaration.name)) {
          if (emitGlobalNames.has(name.text)) {
            yield name;
          }
        }
      }
    } else if (
      isRenameableDeclaration(statement) && statement.name != null &&
      ts.isIdentifier(statement.name) &&
      emitGlobalNames.has(statement.name.text)
    ) {
      yield statement.name;
    }
  }
}

function* getBindingNames(name: ts.BindingName): Generator<ts.Identifier> {
  if (ts.isIdentifier(name)) {
    yield name;
  } else {
    for (const element of name.elements) {
      if (ts.isBindingElement(element)) {
        yield* getBindingNames(element.name);
      }
    }
  }
}

function isRenameableDeclaration(
  node: ts.Node,
): node is
  | ts.FunctionDeclaration
  | ts.ClassDeclaration
  | ts.EnumDeclaration
  | ts.ModuleDeclaration {
  return ts.isFunctionDeclaration(node) || ts.isClassDeclaration(node) ||
    ts.isEnumDeclaration(node) || ts.isModuleDeclaration(node);
}

function getIdentifierNames(sourceFile: ts.SourceFile) {
  const names = new Set<string>();
  visitNode(sourceFile);
  return names;

  function visitNode(node: ts.Node) {
    if (ts.isIdentifier(node)) {
      names.add(node.text);
    }
    ts.forEachChild(node, visitNode);
  }
}

function hasModifier(node: ts.Node, kind: ts.ModifierSyntaxKind) {
  return ts.canHaveModifiers(node) &&
    (ts.getModifiers(node)?.some((m) => m.kind === kind) ?? false);
}
