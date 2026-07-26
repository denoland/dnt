// Copyright 2018-2024 the Deno authors. MIT license.

import { Blob } from "node:buffer";

// node does not have an `alert` global, so this shows the preload
// module ran before the code being tested
Object.assign(globalThis, {
  Blob,
  alert() {},
});
