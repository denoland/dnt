import { path } from "./mod.deps.ts";
import { assertEquals, assertRejects } from "./test.deps.ts";
import { getDntVersion, runCommand, valueToUrl } from "./utils.ts";

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

Deno.test("getDntVersion", () => {
  assertEquals(getDntVersion("https://deno.land/x/dnt@0.1.0/mod.ts"), "0.1.0");
  assertEquals(
    getDntVersion("https://deno.land/x/dnt@20.21.22/mod.ts"),
    "20.21.22",
  );
  assertEquals(getDntVersion("file:///test/mod.ts"), "dev");
});
