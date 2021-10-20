# dnt - Deno to Node Transform

[![deno doc](https://doc.deno.land/badge.svg)](https://doc.deno.land/https/deno.land/x/dnt/mod.ts)

Prototype for a Deno to npm package build tool.

## What does this do?

It takes a Deno module and creates an npm package for use on Node.

There are several steps done in a pipeline:

1. Transforms Deno code to Node/canonical TypeScript including files found by `deno test`.
   1. Rewrites module specifiers.
   1. Injects a [Deno shim](https://github.com/denoland/deno.ns) for any `Deno` namespace usages.
   1. Downloads remote dependencies and rewrites specifiers to make them local.
   1. Rewrites Skypack and ESM specifiers to a bare specifier and includes these dependencies in a package.json.
   1. Allows mapping any specifier to an npm package.
1. Emits ESM, CommonJS, and TypeScript declaration files along with a *package.json* file.
1. Type checks the final output.
1. Runs the final output in Node through a test runner running all `Deno.test` calls. Deletes the test files when complete.

### Example Build Logs

```
[dnt] Transforming...
[dnt] Running npm install in parallel...
[dnt] Building project...
[dnt] Type checking...
[dnt] Emitting declaration files...
[dnt] Emitting esm module...
[dnt] Emitting cjs module...
[dnt] Running tests...

> test
> node test_runner.js

Running tests in ./cjs/mod.test.js...

test escapeForWithinString ... ok
test escapeChar ... ok

Running tests in ./esm/mod.test.js...

test escapeForWithinString ... ok
test escapeChar ... ok
[dnt] Complete!
```

## Setup

```ts
// ex. scripts/build_npm.ts
import { build } from "https://deno.land/x/dnt/mod.ts";

await build({
  entryPoints: ["./mod.ts"],
  outDir: "./npm",
  typeCheck: true,
  declaration: true,
  test: true,
  package: {
    // package.json properties
    name: "my-package",
    version: Deno.args[0],
    description: "My package.",
    license: "MIT",
    repository: {
      type: "git",
      url: "git+https://github.com/dsherret/my-package.git",
    },
    bugs: {
      url: "https://github.com/dsherret/my-package/issues",
    },
  },
  // optional specifier to npm package mappings
  mappings: {
    "https://deno.land/x/code_block_writer@10.1.1/mod.ts": {
      name: "code-block-writer",
      version: "^10.1.1",
    },
  },
});
```

```bash
# run script
deno run --allow-read --allow-write --allow-net --allow-run scripts/build_npm.ts 0.1.0

# go to output directory and publish
cd npm
npm publish
```

## JS API Example

For only the Deno to canonical TypeScript transform which can be useful for
bundlers, use the following:

```ts
// docs: https://doc.deno.land/https/deno.land/x/dnt/transform.ts
import { transform } from "https://deno.land/x/dnt/transform.ts";

const outputResult = await transform({
  entryPoints: ["./mod.ts"],
  testEntryPoints: ["./mod.test.ts"],
  shimPackageName: "deno.ns",
  // mappings: {}, // optional specifier mappings
});
```

## Rust API Example

```rust
use std::path::PathBuf;

use deno_node_transform::ModuleSpecifier;
use deno_node_transform::transform;
use deno_node_transform::TransformOptions;

let output_result = transform(TransformOptions {
  entry_points: vec![ModuleSpecifier::from_file_path(PathBuf::from("./mod.ts")).unwrap()],
  test_entry_points: vec![ModuleSpecifier::from_file_path(PathBuf::from("./mod.test.ts")).unwrap()],
  shim_package_name: "deno.ns".to_string(),
  loader: None, // use the default loader
  specifier_mappings: None,
}).await?;
```
