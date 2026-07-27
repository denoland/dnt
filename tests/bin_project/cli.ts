// Copyright 2018-2024 the Deno authors. MIT license.

import { shared } from "./shared.ts";

// a top level await is fine because a binary isn't in the script output
await Promise.resolve();

console.log(shared);
