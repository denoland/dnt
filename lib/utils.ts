// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import * as deps from "./mod.deps.ts";
import { path } from "./mod.deps.ts";

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

export function runNpmCommand({ bin, args, cwd }: {
  bin: string;
  args: string[];
  cwd: string;
}) {
  return runCommand({
    cmd: [bin, ...args],
    cwd,
  });
}

export async function runCommand(opts: {
  cmd: string[];
  cwd: string;
}) {
  const cmd = getCmd();
  await Deno.permissions.request({ name: "run", command: cmd[0] });

  try {
    const process = Deno.run({
      cmd,
      cwd: opts.cwd,
      stderr: "inherit",
      stdout: "inherit",
      stdin: "inherit",
    });

    try {
      const status = await process.status();
      if (!status.success) {
        throw new Error(
          `${opts.cmd.join(" ")} failed with exit code ${status.code}`,
        );
      }
    } finally {
      process.close();
    }
  } catch (err) {
    // won't happen on Windows, but that's ok because cmd outputs
    // a message saying that the command doesn't exist
    if (err instanceof Deno.errors.NotFound) {
      throw new Error(
        `Could not find command '${
          opts.cmd[0]
        }'. Ensure it is available on the path.`,
        { cause: err },
      );
    } else {
      throw err;
    }
  }

  function getCmd() {
    const cmd = [...opts.cmd];
    if (Deno.build.os === "windows") {
      return ["cmd", "/c", ...opts.cmd];
    } else {
      return cmd;
    }
  }
}

export function standardizePath(fileOrDirPath: string) {
  if (fileOrDirPath.startsWith("file:")) {
    return path.fromFileUrl(fileOrDirPath);
  }
  return path.resolve(fileOrDirPath);
}
