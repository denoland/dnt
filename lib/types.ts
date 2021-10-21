// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

export interface PackageJsonObject {
  name: string;
  version: string;
  [propertyName: string]: any;
}
