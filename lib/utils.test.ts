import * as path from "@std/path";
import { assertEquals, assertRejects } from "@std/assert";
import { getDntVersion, glob, runCommand, valueToUrl } from "./utils.ts";

Deno.test("glob should not search node_modules or excluded dirs", async () => {
  const rootDir = await Deno.makeTempDir();
  try {
    const testFilePaths = [
      ["mod.test.ts"],
      ["sub", "mod.test.ts"],
      ["node_modules", "pkg", "mod.test.ts"],
      ["sub", "node_modules", "pkg", "mod.test.ts"],
      ["npm", "mod.test.ts"],
    ];
    for (const filePath of testFilePaths) {
      const absPath = path.join(rootDir, ...filePath);
      await Deno.mkdir(path.dirname(absPath), { recursive: true });
      await Deno.writeTextFile(absPath, "");
    }

    const paths = await glob({
      pattern: "**/*.test.ts",
      rootDir,
      excludeDirs: [path.join(rootDir, "npm")],
    });

    assertEquals(
      paths.map((p) => path.relative(rootDir, p)).sort(),
      ["mod.test.ts", path.join("sub", "mod.test.ts")],
    );
  } finally {
    await Deno.remove(rootDir, { recursive: true });
  }
});

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
  assertEquals(valueToUrl("jsr:@scope/package"), "jsr:@scope/package");
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
  assertEquals(getDntVersion("https://jsr.io/@deno/dnt/1.2.3/mod.ts"), "1.2.3");
  assertEquals(getDntVersion("file:///test/mod.ts"), "dev");
});
