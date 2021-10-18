import { glob } from "./mod.deps.ts";

// * named `test.{ts, tsx, js, mjs, jsx}`,
// * or ending with `.test.{ts, tsx, js, mjs, jsx}`,
// * or ending with `_test.{ts, tsx, js, mjs, jsx}`

/** Gets the test files found in the provided root dir path. */
export async function getTestFilePaths(options: {
  rootDir: string;
  excludeDirs: string[];
}) {
  const testFilePaths: string[] = [];
  const pattern =
    "**/{test.{ts,tsx,js,mjs,jsx},*.test.{ts,tsx,js,mjs,jsx},*_test.{ts,tsx,js,mjs,jsx}}";
  const entries = glob.expandGlob(pattern, {
    root: options.rootDir,
    extended: true,
    globstar: true,
    exclude: options.excludeDirs,
  });
  for await (const entry of entries) {
    if (entry.isFile) {
      testFilePaths.push(entry.path);
    }
  }
  return testFilePaths;
}
