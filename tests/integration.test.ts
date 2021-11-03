// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import { assertEquals } from "https://deno.land/std@0.109.0/testing/asserts.ts";
import { build, BuildOptions } from "../mod.ts";

Deno.test("should build", async () => {
  await runTest({
    entryPoints: ["mod.ts"],
    outDir: "./npm",
    package: {
      name: "add",
      version: "1.0.0",
    },
  }, (output) => {
    assertEquals(output.packageJson, {
      name: "add",
      version: "1.0.0",
      main: "./umd/mod.js",
      module: "./esm/mod.js",
      exports: {
        ".": {
          import: "./esm/mod.js",
          require: "./umd/mod.js",
          types: "./types/mod.d.ts",
        },
      },
      scripts: {
        test: "node test_runner.js",
      },
      types: "./types/mod.d.ts",
      dependencies: {
        tslib: "2.3.1",
      },
      devDependencies: {
        "@types/node": "16.11.1",
        chalk: "4.1.2",
        "deno.ns": "0.6.3",
      },
    });
    assertEquals(
      output.npmIgnore,
      `esm/mod.test.js
umd/mod.test.js
esm/deps/deno_land_std_0_109_0/fmt/colors.js
umd/deps/deno_land_std_0_109_0/fmt/colors.js
esm/deps/deno_land_std_0_109_0/testing/_diff.js
umd/deps/deno_land_std_0_109_0/testing/_diff.js
esm/deps/deno_land_std_0_109_0/testing/asserts.js
umd/deps/deno_land_std_0_109_0/testing/asserts.js
test_runner.js
`,
    );
  });
});

export interface Output {
  packageJson: any;
  npmIgnore: string;
}

async function runTest(
  options: BuildOptions,
  checkOutput: (output: Output) => (Promise<void> | void),
) {
  const originalCwd = Deno.cwd();
  Deno.chdir("./tests/test_project");
  try {
    await build(options);
    const packageJson = JSON.parse(
      Deno.readTextFileSync(options.outDir + "/package.json"),
    );
    const npmIgnore = Deno.readTextFileSync(options.outDir + "/.npmignore");
    await checkOutput({
      packageJson,
      npmIgnore,
    });
  } finally {
    Deno.removeSync(options.outDir, { recursive: true });
    Deno.chdir(originalCwd);
  }
}
