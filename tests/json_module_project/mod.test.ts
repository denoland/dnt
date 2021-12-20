import { getDynamicOutput, getOutput } from "./mod.ts";

Deno.test("should get output", () => {
  if (getOutput() !== 5) {
    throw new Error("Was not expected output.");
  }
});

Deno.test("should get dynamic output", async () => {
  if ((await getDynamicOutput()) !== 5) {
    throw new Error("Was not expected output.");
  }
});
