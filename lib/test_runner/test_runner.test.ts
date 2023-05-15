// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { runTestDefinitions } from "./test_runner.ts";
import { assertEquals, assertRejects } from "../test.deps.ts";
import { wildcardAssertEquals } from "../test_utils.ts";

Deno.test("no test definitions", async () => {
  const context = getContext();
  await runTestDefinitions([], context);
  assertEquals(context.output, "");
});

Deno.test("failing test definitions", async () => {
  const context = getContext();
  await assertRejects(
    async () => {
      await runTestDefinitions([{
        name: "case 1",
        fn: () => {
          throw new Error("ERROR");
        },
      }, {
        name: "case 2",
        fn: async (t) => {
          if (t.origin !== "file:///file.ts") {
            throw new Error("Origin not equal.");
          }
          if (t.parent !== undefined) {
            throw new Error("Parent should have been undefined");
          }
          await t.step("inner 1", async (tInner) => {
            if (t !== tInner.parent) {
              throw new Error("Parent should have equaled parent.");
            }
            await tInner.step("fail 1", () => {
              throw new Error("FAIL");
            });
            await tInner.step("success 1", () => {});
          });
        },
      }], context);
    },
    Error,
    "Exit code 1 thrown.",
  );
  wildcardAssertEquals(
    context.output,
    `test case 1 ... RfailR
test case 2 ...
  test inner 1 ...
    test fail 1 ...
      Error: FAIL
          at [WILDCARD]
    RfailR
    test success 1 ... GokG
  RfailR
RfailR

FAILURES

case 1
  Error: ERROR
      at [WILDCARD]

case 2
  Error: Had failing test step.
      at [WILDCARD]`,
  );
});

Deno.test("Ignored tests and test cases", async () => {
  const context = getContext();
  await runTestDefinitions([{
    name: "Ignored",
    ignore: true,
    fn: () => {
      throw new Error("FAIL");
    },
  }, {
    name: "Other",
    fn: async (t) => {
      await t.step({
        name: "Ignored",
        fn: () => {
          throw new Error("FAIL");
        },
        ignore: true,
      });
    },
  }], context);

  assertEquals(
    context.output,
    `test Ignored ... YignoredY
test Other ...
  test Ignored ... YignoredY
GokG
`,
  );
});

function getContext() {
  let output = "";
  return {
    get output() {
      return output;
    },
    origin: "file:///file.ts",
    pc: {
      red(text: string) {
        return `R${text}R`;
      },
      green(text: string) {
        return `G${text}G`;
      },
      gray(text: string) {
        return `Y${text}Y`;
      },
    },
    process: {
      stdout: {
        write(text: string) {
          output += text;
        },
      },
      exit(code: number) {
        throw new Error(`Exit code ${code} thrown.`);
      },
    },
  };
}
