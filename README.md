# dnt - Deno to Node Transform

[![deno doc](https://doc.deno.land/badge.svg)](https://doc.deno.land/https/deno.land/x/dnt/mod.ts)

Prototype for a Deno to Node/canonical TypeScript transform.

Note: This is not completely working yet. Please don't use it as it will
probably drastically change.

## Setup

```ts
// ex. scripts/create_npm_package.ts
import { build } from "https://deno.land/x/dnt/mod.ts";

await build({
  entryPoints: ["./mod.ts"],
  outDir: "./npm",
  typeCheck: true,
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
    // optional dev dependencies to use
    devDependencies: {
      // if you find it necessary
      "@types/node": "^16.10.3",
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
# run script. This will output an npm package with cjs and mjs distributions bundling remote dependencies
deno run --allow-read --allow-write --allow-net --allow-run scripts/create_npm_package.ts 0.1.0

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
  entryPoint: "./mod.ts",
  shimPackageName: "deno.ns",
});

// inspect outputResult.files and outputResult.dependencies here
```

## Rust API Example

When using TypeScript, the Rust API only transforms from Deno to canonical
TypeScript. You will need to use the TypeScript compiler to do the rest.

```rust
use std::path::PathBuf;

use deno_node_transform::ModuleSpecifier;
use deno_node_transform::transform;
use deno_node_transform::TransformOptions;

let output_result = transform(TransformOptions {
  entry_point: ModuleSpecifier::from_file_path(PathBuf::from("./mod.ts")).unwrap(),
  shim_package_name: "deno.ns".to_string(),
  loader: None, // use the default loader
  specifier_mappings: None,
}).await?;

for output_file in output_result.files.iter() {
  // use these properties on output_file
  output_file.file_path;
  output_file.file_text;
}
```
