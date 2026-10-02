import { callAdd } from "./mod.ts";

Deno.test("wasm module test", () => {
  const add = callAdd();
  if (typeof add !== "function" && typeof add !== "undefined") {
    throw new Error("Unexpected export type");
  }
});
