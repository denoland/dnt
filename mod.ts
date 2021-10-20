// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { outputDiagnostics } from "./lib/compiler.ts";
import { colors, createProjectSync, path, ts } from "./lib/mod.deps.ts";
import { getTestFilePaths } from "./lib/test_files.ts";
import { PackageJsonObject } from "./lib/types.ts";
import { transform, TransformOutput } from "./transform.ts";

export * from "./transform.ts";

export interface BuildOptions {
  /** Entrypoint(s) to the Deno module. Ex. `./mod.ts` */
  entryPoints: (string | URL)[];
  /** Directory to output to. */
  outDir: string;
  /** Type check the output (defaults to true). */
  typeCheck?: boolean;
  /** Collect and run test files (defaults to true). */
  test?: boolean;
  /** Create declaration files (defaults to true). */
  declaration?: boolean;
  /** Keep the test files after tests run. */
  keepTestFiles?: boolean;
  /** Root directory to find test files in. Defaults to the cwd. */
  rootTestDir?: string;
  shimPackage?: {
    name: string;
    version: string;
  };
  /** Specifiers to map from and to. */
  mappings?: {
    [specifier: string]: {
      /** Name of the specifier to map to. */
      name: string;
      /** Version to use in the package.json file.
       *
       * Not specifying a version will exclude it from the package.json file.
       */
      version?: string;
    };
  };
  /** Package.json output. You may override dependencies and dev dependencies in here. */
  package: PackageJsonObject;
}

/** Emits the specified Deno module to an npm package using the TypeScript compiler. */
export async function build(options: BuildOptions): Promise<void> {
  // set defaults
  options = {
    ...options,
    typeCheck: options.typeCheck ?? true,
    test: options.test ?? true,
    declaration: options.declaration ?? true,
  };

  await Deno.permissions.request({ name: "write", path: options.outDir });

  const shimPackage = options.shimPackage ?? {
    name: "deno.ns",
    version: "0.5.0",
  };
  const specifierMappings = options.mappings && Object.fromEntries(
    Object.entries(options.mappings).map(([key, value]) => {
      const lowerCaseKey = key.toLowerCase();
      if (
        !lowerCaseKey.startsWith("http://") &&
        !lowerCaseKey.startsWith("https://")
      ) {
        key = path.toFileUrl(lowerCaseKey).toString();
      }
      return [key, value];
    }),
  );

  log("Transforming...");
  const transformOutput = await transformEntryPoints();
  for (const warning of transformOutput.warnings) {
    warn(warning);
  }

  const createdDirectories = new Set<string>();
  const writeFile = ((filePath: string, fileText: string) => {
    const dir = path.dirname(filePath);
    if (!createdDirectories.has(dir)) {
      Deno.mkdirSync(dir, { recursive: true });
      createdDirectories.add(dir);
    }
    Deno.writeTextFileSync(filePath, fileText);
  });

  createPackageJson();
  createNpmIgnore();

  // npm install in order to prepare for checking TS diagnostics
  log("Running npm install in parallel...");
  const npmInstallPromise = runNpmCommand(["install"]);

  log("Building project...");
  const esmOutDir = path.join(options.outDir, "esm");
  const cjsOutDir = path.join(options.outDir, "cjs");
  const typesOutDir = path.join(options.outDir, "types");
  const project = createProjectSync({
    compilerOptions: {
      outDir: typesOutDir,
      allowJs: true,
      stripInternal: true,
      declaration: true,
      esModuleInterop: false,
      isolatedModules: true,
      useDefineForClassFields: true,
      experimentalDecorators: true,
      jsx: ts.JsxEmit.React,
      jsxFactory: "React.createElement",
      jsxFragmentFactory: "React.Fragment",
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
      module: ts.ModuleKind.ES2015,
      moduleResolution: ts.ModuleResolutionKind.NodeJs,
      target: ts.ScriptTarget.ES2015,
      allowSyntheticDefaultImports: true,
    },
  });

  for (const outputFile of [
    ...transformOutput.main.files,
    ...transformOutput.test.files
  ]) {
    project.createSourceFile(
      path.join(options.outDir, "src", outputFile.filePath),
      outputFile.fileText,
    );
  }

  let program = project.createProgram();

  if (options.typeCheck) {
    await npmInstallPromise; // need to ensure this is done before type checking
    log("Type checking...");
    const diagnostics = ts.getPreEmitDiagnostics(program);
    if (diagnostics.length > 0) {
      outputDiagnostics(diagnostics);
      Deno.exit(1);
    }
  }

  // emit only the .d.ts files
  if (options.declaration) {
    log("Emitting declaration files...");
    emit({ onlyDtsFiles: true });
  }

  // emit the esm files
  log("Emitting esm module...");
  project.compilerOptions.set({
    declaration: false,
    outDir: esmOutDir,
  });
  program = project.createProgram();
  emit();
  writeFile(
    path.join(esmOutDir, "package.json"),
    `{\n  "type": "module"\n}\n`,
  );

  // emit the cjs files
  log("Emitting cjs module...");
  project.compilerOptions.set({
    declaration: false,
    esModuleInterop: true,
    outDir: cjsOutDir,
    module: ts.ModuleKind.CommonJS,
  });
  program = project.createProgram();
  emit();
  writeFile(
    path.join(cjsOutDir, "package.json"),
    `{\n  "type": "commonjs"\n}\n`,
  );

  // ensure this is done before running tests
  await npmInstallPromise;

  if (options.test) {
    log("Running tests...");
    await createTestLauncherScript();
    await runNpmCommand(["run", "test"]);
    if (!options.keepTestFiles) {
      await deleteTestFiles();
    }
  }

  log("Complete!");

  function emit(opts?: { onlyDtsFiles?: boolean }) {
    const emitResult = program.emit(
      undefined,
      (filePath, data, writeByteOrderMark) => {
        if (writeByteOrderMark) {
          data = "\uFEFF" + data;
        }
        writeFile(filePath, data);
      },
      undefined,
      opts?.onlyDtsFiles,
    );

    if (emitResult.diagnostics.length > 0) {
      outputDiagnostics(emitResult.diagnostics);
      Deno.exit(1);
    }
  }

  function createPackageJson() {
    const entryPointPath = transformOutput
      .main.entryPoints[0]
      .replace(/\.ts$/i, ".js");
    const entryPointDtsFilePath = transformOutput
      .main.entryPoints[0]
      .replace(/\.ts$/i, ".d.ts");
    const dependencies = {
      // add dependencies from transform
      ...Object.fromEntries(
        transformOutput.main.dependencies.map((d) => [d.name, d.version]),
      ),
      // add specifier mappings to dependencies
      ...(specifierMappings && Object.fromEntries(
        Object.values(specifierMappings)
          .filter((v) => v.version)
          .map((value) => [value.name, value.version]),
      )) ?? {},
      // add shim
      ...(transformOutput.main.shimUsed
        ? {
          [shimPackage.name]: shimPackage.version,
        }
        : {}),
      // override with specified dependencies
      ...(options.package.dependencies ?? {}),
    };
    const devDependencies = options.test
      ? ({
        ...(!Object.keys(dependencies).includes("chalk")
          ? {
            "chalk": "4.1.2",
          }
          : {}),
        ...(!Object.keys(dependencies).includes("@types/node") &&
            (transformOutput.main.shimUsed || transformOutput.test.shimUsed)
          ? {
            "@types/node": "16.11.1",
          }
          : {}),
        // add dependencies from transform
        ...Object.fromEntries(
          transformOutput.test.dependencies.map((d) => [d.name, d.version]) ?? [],
        ),
        // add shim if not in dependencies
        ...(transformOutput.test.shimUsed &&
            !Object.keys(dependencies).includes(shimPackage.name)
          ? {
            [shimPackage.name]: shimPackage.version,
          }
          : {}),
        // override with specified dependencies
        ...(options.package.devDependencies ?? {}),
      })
      : options.package.devDependencies;
    const scripts = options.test
      ? ({
        test: "node test_runner.js",
        // override with specified scripts
        ...(options.package.scripts ?? {}),
      })
      : options.package.scripts;

    const packageJsonObj = {
      ...options.package,
      module: options.package.module ?? `./esm/${entryPointPath}`,
      main: options.package.main ?? `./cjs/${entryPointPath}`,
      types: options.package.types ?? `./types/${entryPointDtsFilePath}`,
      exports: {
        ...(options.package.exports ?? {}),
        ".": {
          import: `./esm/${entryPointPath}`,
          require: `./cjs/${entryPointPath}`,
          types: options.package.types ?? `./types/${entryPointDtsFilePath}`,
          ...(options.package.exports?.["."] ?? {}),
        },
      },
      scripts,
      dependencies,
      devDependencies,
    };
    writeFile(
      path.join(options.outDir, "package.json"),
      JSON.stringify(packageJsonObj, undefined, 2),
    );
  }

  function createNpmIgnore() {
    if (!options.test) {
      return;
    }

    const fileText = Array.from(getTestFileNames()).join("\n");
    writeFile(
      path.join(options.outDir, ".npmignore"),
      fileText,
    );
  }

  async function deleteTestFiles() {
    for (const file of getTestFileNames()) {
      await Deno.remove(path.join(options.outDir, file));
    }
  }

  function* getTestFileNames() {
    for (const file of transformOutput.test.files) {
      const filePath = file.filePath.replace(/\.ts$/i, ".js");
      yield `./esm/${filePath}`;
      yield `./cjs/${filePath}`;
    }
    yield "./test_runner.js";
  }

  async function transformEntryPoints(): Promise<TransformOutput> {
    return transform({
      entryPoints: options.entryPoints,
      testEntryPoints: options.test ? await getTestFilePaths({
        rootDir: options.rootTestDir ?? Deno.cwd(),
        excludeDirs: [options.outDir],
      }) : [],
      shimPackageName: shimPackage.name,
      specifierMappings: specifierMappings && Object.fromEntries(
        Object.entries(specifierMappings).map(([key, value]) => {
          return [key, value.name];
        }),
      ),
    });
  }

  function log(message: string) {
    console.log(`[dnt] ${message}`);
  }

  function warn(message: string) {
    console.warn(colors.yellow(`[dnt] ${message}`));
  }

  async function createTestLauncherScript() {
    let fileText = `const chalk = require("chalk");\n`;
    if (transformOutput.test.shimUsed) {
      fileText += `const denoShim = require("${shimPackage.name}");\n` +
        `const { testDefinitions } = require("${shimPackage.name}/test-internals");\n\n`;
    }

    fileText += "const filePaths = [\n";
    for (const entryPoint of transformOutput.test.entryPoints) {
      fileText += `  "${entryPoint.replace(/\.ts$/, ".js")}",\n`;
    }
    fileText += "];\n\n";

    fileText += `async function main() {
  for (const [i, filePath] of filePaths.entries()) {
    const cjsPath = "./cjs/" + filePath;
    if (i > 0) {
      console.log("");
    }
    console.log("Running tests in " + chalk.underline(cjsPath) + "...\\n");
    require(cjsPath);
    await runTestDefinitions();
    const esmPath = "./esm/" + filePath;
    console.log("\\nRunning tests in " + chalk.underline(esmPath) + "...\\n");
    await import(esmPath);
    await runTestDefinitions();
  }
}\n\n`;
    if (transformOutput.test.shimUsed) {
      fileText += `${getRunTestDefinitionsCode()}\n\n`;
    }
    fileText += "main();\n";

    writeFile(
      path.join(options.outDir, "test_runner.js"),
      fileText,
    );
  }

  async function runNpmCommand(args: string[]) {
    const cmd = getCmd();
    await Deno.permissions.request({ name: "run", command: cmd[0] });
    const process = Deno.run({
      cmd,
      cwd: options.outDir,
      stderr: "inherit",
      stdout: "inherit",
      stdin: "inherit",
    });

    try {
      const status = await process.status();
      if (!status.success) {
        throw new Error(
          `npm ${args.join(" ")} failed with exit code ${status.code}`,
        );
      }
    } finally {
      process.close();
    }

    function getCmd() {
      const cmd = ["npm", ...args];
      if (Deno.build.os === "windows") {
        return ["cmd", "/c", ...cmd];
      } else {
        return cmd;
      }
    }
  }
}

function getRunTestDefinitionsCode() {
  // todo: extract out for unit testing
  return `
async function runTestDefinitions() {
  const currentDefinitions = testDefinitions.splice(0, testDefinitions.length);
  const testFailures = [];
  for (const definition of currentDefinitions) {
    process.stdout.write("test " + definition.name + " ...");
    if (definition.ignored) {
     process.stdout.write(" ignored\\n");
     continue;
    }
    const context = getTestContext();
    let pass = false;
    try {
      await definition.fn(context);
      if (context.hasFailingChild) {
        testFailures.push({ name: definition.name, err: new Error("Had failing test step.") });
      } else {
        pass = true;
      }
    } catch (err) {
      testFailures.push({ name: definition.name, err });
    }
    const testStepOutput = context.getOutput();
    if (testStepOutput.length > 0) {
      process.stdout.write(testStepOutput);
    } else {
      process.stdout.write(" ");
    }
    process.stdout.write(getStatusText(pass ? "ok" : "fail"));
    process.stdout.write("\\n");
  }

  if (testFailures.length > 0) {
    console.log("\\nFAILURES\\n");
    for (const failure of testFailures) {
      console.log(failure.name);
      console.log(indentText(failure.err, 1));
      console.log("");
    }
    process.exit(1);
  }
}

function getTestContext() {
  return {
    name: undefined,
    status: "ok",
    children: [],
    get hasFailingChild() {
      return this.children.some(c => c.status === "fail" || c.status === "pending");
    },
    getOutput() {
      let output = "";
      if (this.name) {
        output += "test " + this.name + " ...";
      }
      if (this.children.length > 0) {
        output += "\\n" + this.children.map(c => indentText(c.getOutput(), 1)).join("\\n") + "\\n";
      } else if (!this.err) {
        output += " ";
      }
      if (this.name && this.err) {
        output += "\\n";
      }
      if (this.err) {
        output += indentText(this.err.toString(), 1);
        if (this.name) {
          output += "\\n";
        }
      }
      if (this.name) {
        output += getStatusText(this.status);
      }
      return output;
    },
    async step(nameOrTestDefinition, fn) {
      const definition = getDefinition();

      const context = getTestContext();
      context.status = "pending";
      context.name = definition.name;
      context.status = "pending";
      this.children.push(context);

      if (definition.ignored) {
        context.status = "ignored";
        return false;
      }

      try {
        await definition.fn(context);
        context.status = "ok";
        if (context.hasFailingChild) {
          context.status = "fail";
          return false;
        }
        return true;
      } catch (err) {
        context.status = "fail";
        context.err = err;
        return false;
      }

      function getDefinition() {
        if (typeof nameOrTestDefinition === "string") {
          if (!(fn instanceof Function)) {
            throw new TypeError("Expected function for second argument.");
          }
          return {
            name: nameOrTestDefinition,
            fn,
          };
        } else if (typeof nameOrTestDefinition === "object") {
          return nameOrTestDefinition;
        } else {
          throw new TypeError(
            "Expected a test definition or name and function.",
          );
        }
      }
    }
  };
}

function getStatusText(status) {
  switch (status) {
    case "ok":
      return chalk.green(status);
    case "fail":
    case "pending":
      return chalk.red(status);
    case "ignore":
      return chalk.gray(status);
    default:
      return status;
  }
}

function indentText(text, indentLevel) {
  if (text === undefined) {
    text = "[undefined]";
  } else if (text === null) {
    text = "[null]";
  } else {
    text = text.toString();
  }
  return text.split(/\\r?\\n/).map(line => "  ".repeat(indentLevel) + line).join("\\n");
}
`.trim();
}
