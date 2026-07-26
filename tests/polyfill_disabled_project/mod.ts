// Copyright 2018-2024 the Deno authors. MIT license.

// `import.meta.url` is standard, so this builds without the import.meta
// ponyfill as long as only an ES module is emitted
export const url = import.meta.url;
