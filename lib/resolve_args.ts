import { ts } from "./mod.deps.ts"
import { ParsedArgs } from "./args.ts";
import { DiagnosticsError } from "./compiler.ts";
import { PackageJsonObject } from "./types.ts";

export interface ResolvedArgs {
  entryPoint: string;
  shimPackage: string | undefined;
  typeCheck: boolean;
  package: PackageJsonObject;
  outDir: string;
}

export function resolveArgs(cliArgs: ParsedArgs): ResolvedArgs {
  const config = getConfig();
  return {
    entryPoint: getEntryPoint(),
    package: getPackage(),
    typeCheck: cliArgs.typeCheck ?? config?.typeCheck ?? false,
    shimPackage: cliArgs.shimPackage ?? config?.shimPackage,
    outDir: getOutDir(),
  };

  function getConfig() {
    if (!cliArgs.config) {
      return undefined;
    }
    const filePath = cliArgs.config;
    const fileText = Deno.readTextFileSync(filePath);
    return parseConfig(filePath, fileText);
  }

  function getEntryPoint() {
    if (cliArgs.entryPoint) {
      return cliArgs.entryPoint;
    } else if (config?.entryPoint) {
      return config.entryPoint;
    } else {
      throw new Error(`Please provide an entry point (ex. mod.ts)`)
    }
  }

  function getOutDir() {
    if (cliArgs.outDir) {
      return cliArgs.outDir;
    } else if (config?.outDir) {
      return config.outDir;
    } else {
      throw new Error(`Please provide an outDir directory (ex. dist)`)
    }
  }

  function getPackage() {
    const packageObj = (config?.package ?? {}) as PackageJsonObject;
    if (cliArgs.packageName) {
      packageObj.name = cliArgs.packageName;
    }
    if (cliArgs.packageVersion) {
      packageObj.version = cliArgs.packageVersion;
    }
    if (!packageObj.name) {
      throw new Error("You must specify a package name either by providing a `packageName` CLI argument or specifying one in the config file's package object.");
    }
    if (!packageObj.version) {
      throw new Error("You must specify a package version either by providing a `packageVersion` CLI argument or specifying one in the config file's package object.");
    }
    return packageObj;
  }
}

function parseConfig(filePath: string, fileText: string) {
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
    outDir: getValueProperty("outDir", "string") as string | undefined
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
}
