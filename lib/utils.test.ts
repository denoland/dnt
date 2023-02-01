import { path } from "./mod.deps.ts";
import { assertEquals, assertRejects } from "./test.deps.ts";
import { runCommand, valueToUrl } from "./utils.ts";

Deno.test({
  name: "should error when command doesn't exist",
  ignore: Deno.build.os === "windows",
  async fn() {
    const commandName = "somenonexistentcommandforsure";
    await assertRejects(
      () =>
        runCommand({
          cmd: [commandName],
          cwd: Deno.cwd(),
        }),
      Error,
      `Could not find command '${commandName}'. Ensure it is available on the path.`,
    );
  },
});

Deno.test("valueToUrl", () => {
  assertEquals(valueToUrl("npm:test"), "npm:test");
  assertEquals(valueToUrl("node:path"), "node:path");
  assertEquals(valueToUrl("https://deno.land"), "https://deno.land");
  assertEquals(valueToUrl("http://deno.land"), "http://deno.land");
  assertEquals(
    valueToUrl("test"),
    path.toFileUrl(path.resolve("test")).toString(),
  );
  assertEquals(valueToUrl("file:///test"), "file:///test");
});
