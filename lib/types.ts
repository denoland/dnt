// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

export interface PackageJsonObject {
  name: string;
  version: string;
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
  | "Latest";
