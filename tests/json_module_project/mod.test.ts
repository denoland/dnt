import { getDynamicOutput, getOutput } from "./mod.ts";

Deno.test("should get output", () => {
  // @ts-expect-error: not assignable to string
  const _err: string = getOutput();
  // is assignable to number
  const value: number = getOutput();

  if (value !== 5) {
    throw new Error("Was not expected output.");
  }
});

Deno.test("should get dynamic output", async () => {
  if ((await getDynamicOutput()) !== 5) {
    throw new Error("Was not expected output.");
  }
});
