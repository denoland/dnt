import * as deps from "./mod.deps.ts";

/** Gets the files found in the provided root dir path based on the glob. */
export async function glob(options: {
  pattern: string;
  rootDir: string;
  excludeDirs: string[];
}) {
  const paths: string[] = [];
  const entries = deps.glob.expandGlob(options.pattern, {
    root: options.rootDir,
    extended: true,
    globstar: true,
    exclude: options.excludeDirs,
  });
  for await (const entry of entries) {
    if (entry.isFile) {
      paths.push(entry.path);
    }
  }
  return paths;
}
