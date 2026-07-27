import { build } from "./mod.ts";
Deno.chdir("C:/Users/david/AppData/Local/Temp/claude/V--dnt/9186f2c7-cbce-4bd5-a96d-23329d792f3d/scratchpad/triage/binproj");
const nameless = Deno.args[0] === "nameless";
await build({
  entryPoints: nameless
    ? [{ kind: "bin", path: "./cli.ts" }]
    : [{ name: ".", path: "./mod.ts" }, { kind: "bin", name: "mycli", path: "./cli.ts" }],
  outDir: "./npm",
  shims: {},
  test: false,
  typeCheck: false,
  skipNpmInstall: true,
  esModule: Deno.args[0] !== "cjsonly",
  scriptModule: "cjs",
  package: { name: "binpkg", version: "0.0.0" },
});
