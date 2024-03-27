// Copyright 2018-2024 the Deno authors. MIT license.

export interface Picocolors {
  green(text: string): string;
  red(text: string): string;
  gray(text: string): string;
}

export interface NodeProcess {
  stdout: {
    write(text: string): void;
  };
  exit(code: number): number;
}

export interface RunTestDefinitionsOptions {
  pc: Picocolors;
  process: NodeProcess;
  /** The file the tests are running in. */
  origin: string;
}

export interface TestDefinition {
  name: string;
  fn: (context: TestContext) => Promise<void> | void;
  only?: boolean;
  ignore?: boolean;
}

export interface TestContext {
  name: string;
  parent: TestContext | undefined;
  origin: string;
  err: any;
  children: TestContext[];
  hasFailingChild: boolean;
  getOutput(): string;
  step(
    nameOrDefinition: string | TestDefinition,
    fn?: (context: TestContext) => void | Promise<void>,
  ): Promise<boolean>;
  status: "ok" | "fail" | "pending" | "ignored";
}

export async function runTestDefinitions(
  testDefinitions: TestDefinition[],
  options: RunTestDefinitionsOptions,
) {
  const testFailures = [];
  const hasOnly = testDefinitions.some((d) => d.only);
  if (hasOnly) {
    testDefinitions = testDefinitions.filter((d) => d.only);
  }
  for (const definition of testDefinitions) {
    options.process.stdout.write("test " + definition.name + " ...");
    if (definition.ignore) {
      options.process.stdout.write(` ${options.pc.gray("ignored")}\n`);
      continue;
    }
    const context = getTestContext(definition, undefined);
    let pass = false;
    try {
      await definition.fn(context);
      if (context.hasFailingChild) {
        testFailures.push({
          name: definition.name,
          err: new Error("Had failing test step."),
        });
      } else {
        pass = true;
      }
    } catch (err) {
      testFailures.push({ name: definition.name, err });
    }
    const testStepOutput = context.getOutput();
    if (testStepOutput.length > 0) {
      options.process.stdout.write(testStepOutput);
    } else {
      options.process.stdout.write(" ");
    }
    options.process.stdout.write(getStatusText(pass ? "ok" : "fail"));
    options.process.stdout.write("\n");
  }

  if (testFailures.length > 0) {
    options.process.stdout.write("\nFAILURES");
    for (const failure of testFailures) {
      options.process.stdout.write("\n\n");
      options.process.stdout.write(failure.name + "\n");
      options.process.stdout.write(
        indentText((failure.err?.stack ?? failure.err).toString(), 1),
      );
    }
    options.process.exit(1);
  } else if (hasOnly) {
    options.process.stdout.write(
      'error: Test failed because the "only" option was used.\n',
    );
    options.process.exit(1);
  }

  function getTestContext(
    definition: TestDefinition,
    parent: TestContext | undefined,
  ): TestContext {
    return {
      name: definition.name,
      parent,
      origin: options.origin,
      /** @type {any} */
      err: undefined,
      status: "ok",
      children: [],
      get hasFailingChild() {
        return this.children.some((c) =>
          c.status === "fail" || c.status === "pending"
        );
      },
      getOutput() {
        let output = "";
        if (this.parent) {
          output += "test " + this.name + " ...";
        }
        if (this.children.length > 0) {
          output += "\n" + this.children.map((c) =>
            indentText(c.getOutput(), 1)
          ).join("\n") + "\n";
        } else if (!this.err) {
          output += " ";
        }
        if (this.parent && this.err) {
          output += "\n";
        }
        if (this.err) {
          output += indentText((this.err.stack ?? this.err).toString(), 1);
          if (this.parent) {
            output += "\n";
          }
        }
        if (this.parent) {
          output += getStatusText(this.status);
        }
        return output;
      },
      async step(nameOrTestDefinition, fn) {
        const definition = getDefinition();

        const context = getTestContext(definition, this);
        context.status = "pending";
        this.children.push(context);

        if (definition.ignore) {
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

        /** @returns {TestDefinition} */
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
      },
    };
  }

  function getStatusText(status: TestContext["status"]) {
    switch (status) {
      case "ok":
        return options.pc.green(status);
      case "fail":
      case "pending":
        return options.pc.red(status);
      case "ignored":
        return options.pc.gray(status);
      default: {
        const _assertNever: never = status;
        return status;
      }
    }
  }

  function indentText(text: string, indentLevel: number) {
    if (text === undefined) {
      text = "[undefined]";
    } else if (text === null) {
      text = "[null]";
    } else {
      text = text.toString();
    }
    return text.split(/\r?\n/)
      .map((line) => "  ".repeat(indentLevel) + line)
      .join("\n");
  }
}
