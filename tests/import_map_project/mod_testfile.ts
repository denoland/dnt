// this file is not named .test so it won't be picked up by regular `deno test` in root dir of dnt
import { add } from "./mod.ts";

Deno.test("should add", () => {
  if (add(1, 2) !== 3) {
    throw new Error("Didn't equal.");
  }
});
