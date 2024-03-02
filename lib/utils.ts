// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { expandGlob } from "@std/fs/expand_glob";
import * as path from "@std/path";

/** Gets the files found in the provided root dir path based on the glob. */
export async function glob(options: {
  pattern: string;
  rootDir: string;
  excludeDirs: string[];
}) {
  const paths: string[] = [];
  const entries = expandGlob(options.pattern, {
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
  const [cmd, ...args] = getCmd();
  await Deno.permissions.request({ name: "run", command: cmd });

  try {
    const process = new Deno.Command(cmd, {
      args,
      cwd: opts.cwd,
      stderr: "inherit",
      stdout: "inherit",
      stdin: "inherit",
    });

    const output = await process.output();
    if (!output.success) {
      throw new Error(
        `${opts.cmd.join(" ")} failed with exit code ${output.code}`,
      );
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

export function valueToUrl(value: string) {
  const lowerCaseValue = value.toLowerCase();
  if (
    lowerCaseValue.startsWith("http:") ||
    lowerCaseValue.startsWith("https:") ||
    lowerCaseValue.startsWith("npm:") ||
    lowerCaseValue.startsWith("node:") ||
    lowerCaseValue.startsWith("file:")
  ) {
    return value;
  } else {
    return path.toFileUrl(path.resolve(value)).toString();
  }
}

export function getDntVersion(url = import.meta.url) {
  return /\/dnt@([0-9]+\.[0-9]+\.[0-9]+)\//.exec(url)?.[1] ?? "dev";
}
