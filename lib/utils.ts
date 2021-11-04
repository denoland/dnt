// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

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

export async function runNpmCommand({ args, cwd }: {
  args: string[];
  cwd: string;
}) {
  const cmd = getCmd();
  await Deno.permissions.request({ name: "run", command: cmd[0] });
  const process = Deno.run({
    cmd,
    cwd,
    stderr: "inherit",
    stdout: "inherit",
    stdin: "inherit",
  });

  try {
    const status = await process.status();
    if (!status.success) {
      throw new Error(
        `npm ${args.join(" ")} failed with exit code ${status.code}`,
      );
    }
  } finally {
    process.close();
  }

  function getCmd() {
    const cmd = ["npm", ...args];
    if (Deno.build.os === "windows") {
      return ["cmd", "/c", ...cmd];
    } else {
      return cmd;
    }
  }
}
