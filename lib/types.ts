// Copyright 2018-2024 the Deno authors. MIT license.

interface PackageJsonPerson {
  name: string;
  email?: string;
  url?: string;
}

interface PackageJsonBugs {
  url?: string;
  email?: string;
}

/**
 * Based on version 9.6.6
 */
export interface PackageJson {
  name: string;
  version: string;
  description?: string;
  keywords?: string[];
  homepage?: string;
  bugs?: PackageJsonBugs | string;
  /**
   * Check https://spdx.org/licenses/ for valid licences
   */
  license?: "MIT" | "ISC" | "UNLICENSED" | string;
  author?: PackageJsonPerson | string;
  contributors?: (PackageJsonPerson | string)[];
  main?: string;
  types?: string;
  scripts?: { [key: string]: string };
  repository?: string | { type: string; url: string; directory?: string };
  dependencies?: { [packageName: string]: string };
  devDependencies?: { [packageName: string]: string };
  peerDependencies?: { [packageName: string]: string };
  bundleDependencies?: { [packageName: string]: string };
  optionalDependencies?: { [packageName: string]: string };
  engines?: { [engineName: string]: string };
  /**
   * A list of os like "darwin", "linux", "win32", OS names can be prefix by a "!"
   */
  os?: string[];
  /**
   * A list of cpu like "x64", "ia32", "arm", "mips", CPU names can be prefix by a "!"
   */
  cpu?: string[];
  private?: boolean;
  /**
   * rest of the fields
   */
  [propertyName: string]: any;
}

// NOTICE: make sure to update `ScriptTarget` in the rust code when changing the names on this
// todo(dsherret): code generate this from the Rust code to prevent out of sync issues

/** Version of ECMAScript to compile the code to. */
export type ScriptTarget =
  | "ES3"
  | "ES5"
  | "ES2015"
  | "ES2016"
  | "ES2017"
  | "ES2018"
  | "ES2019"
  | "ES2020"
  | "ES2021"
  | "ES2022"
  | "ES2023"
  | "Latest";
