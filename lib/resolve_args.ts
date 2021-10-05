import { ts } from "./mod.deps.ts"
import { ParsedArgs } from "./args.ts";
import { DiagnosticsError } from "./compiler.ts";

export interface ResolvedArgs {
  compilerOptions: ts.CompilerOptions;
  package: object | undefined;
  entryPoint: string;
  typeCheck: boolean;
  shimPackage: string | undefined;
  packageVersion: string | undefined;
}

export function resolveArgs(cliArgs: ParsedArgs): ResolvedArgs {
  const config = getConfig();
  return {
    compilerOptions: config?.compilerOptions ?? cliArgs.compilerOptions,
    package: getPackage(),
    entryPoint: getEntryPoint(),
    typeCheck: config?.typeCheck ?? cliArgs.typeCheck ?? false,
    shimPackage: config?.shimPackage ?? cliArgs.shimPackage,
    packageVersion: cliArgs.packageVersion,
  };

  function getConfig() {
    if (!cliArgs.config) {
      return undefined;
    }
    const filePath = cliArgs.config;
    const fileText = Deno.readTextFileSync(filePath);
    return parseConfig(filePath, fileText, cliArgs.compilerOptions);
  }

  function getEntryPoint() {
    if (cliArgs.entryPoint) {
      return cliArgs.entryPoint;
    } else if (config?.entryPoint) {
      return config.entryPoint;
    } else {
      throw new Error(`Please provide an entry point (ex. mod.ts).`)
    }
  }

  function getPackage() {
    const packageObj = config?.package;
    if (!packageObj) {
      return undefined;
    }
    if (cliArgs.packageVersion) {
      (packageObj as any).version = cliArgs.packageVersion;
    }
    return packageObj;
  }
}

function parseConfig(filePath: string, fileText: string, existingCompilerOptions: ts.CompilerOptions | undefined) {
  // use this function from the compiler API in order to parse JSONC
  const configFile = ts.parseJsonText(filePath, fileText);
  const parseDiagnostics: ts.Diagnostic[] = (configFile as any).parseDiagnostics ?? [];
  if (parseDiagnostics.length > 0) {
    throw new DiagnosticsError(parseDiagnostics);
  }
  const configFileObj = ts.convertToObject(configFile, []);
  const rootExpression = configFile.statements[0]?.expression;
  if (!rootExpression || rootExpression.kind !== ts.SyntaxKind.ObjectLiteralExpression) {
    throw new Error("The specified config file must contain a JSON object.");
  }

  return {
    // todo: type these better
    entryPoint: getValueProperty("entryPoint", "string") as string | undefined,
    typeCheck: getValueProperty("typeCheck", "boolean") as boolean | undefined,
    package: getValueProperty("package", "object") as object | undefined,
    shimPackage: getValueProperty("shimPackage", "string") as string | undefined,
    compilerOptions: getCompilerOptions(rootExpression),
  };

  function getValueProperty(name: string, kind: "string" | "boolean" | "object") {
    const property = configFileObj[name];
    if (!property) {
      return undefined;
    }
    if (typeof property !== kind) {
      throw new Error(`Expected ${kind} property for ${name}.`);
    }
    return property;
  }

  function getCompilerOptions(rootObject: ts.ObjectLiteralExpression) {
    const configOptionsProperty = getPropertyByName(rootObject, "compilerOptions");
    if (!configOptionsProperty) {
      return existingCompilerOptions;
    }

    const configOptionsResult = ts.parseConfigFileTextToJson(filePath, configOptionsProperty.initializer.getText(configFile))
    if (configOptionsResult.error) {
      throw new DiagnosticsError([configOptionsResult.error]);
    }
    const configResult = ts.parseJsonConfigFileContent(configOptionsResult.config, {
      fileExists(_path) {
        return false;
      },
      readFile(_path) {
        return undefined;
      },
      readDirectory() {
        return [];
      },
      useCaseSensitiveFileNames: true,
    }, Deno.cwd(), existingCompilerOptions, filePath);
    const errors = configResult.errors.filter(e => e.code !== 18003);
    if (errors.length > 0) {
      throw new DiagnosticsError(errors);
    }
    console.log(configResult.options);
    return configResult.options;
  }

  function getPropertyByName(rootObject: ts.ObjectLiteralExpression, name: string) {
    return rootObject.properties.find(p => {
      if (!p.name || p.name.kind !== ts.SyntaxKind.StringLiteral) {
        return false;
      }
      return p.name.text === name;
    }) as ts.PropertyAssignment;
  }
}