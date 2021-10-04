# dnt - Deno to Node Transform

[![deno doc](https://doc.deno.land/badge.svg)](https://doc.deno.land/https/deno.land/x/dnt/mod.ts)

Prototype for a Deno to Node/canonical TypeScript transform.

## CLI Example

```bash
# install
deno install --allow-read --allow-write --allow-net -n dnt https://deno.land/x/dnt/cli.ts

# clone a Deno-first repo
git clone https://github.com/dsherret/code-block-writer.git
cd code-block-writer

# run tool and output to ./code-block-writer/npm/dist (uses tsc CLI flags)
dnt mod.ts --target ES6 --outDir ./npm/dist --declaration

# go to output directory, run tsc, and publish
cd npm
npm publish
```

## Wasm API Example

To emit the Deno-first sources to code that can be consumed in Node.js, use the
`emit` function:

```ts
// docs: https://doc.deno.land/https/deno.land/x/dnt/mod.ts
import { emit } from "https://deno.land/x/dnt/mod.ts";

await emit({
  compilerOptions: {
    outDir: "./dist",
  },
  entryPoint: "./mod.ts",
  shimPackageName: "deno-shim-package-name",
  typeCheck: false,
});
```

For only the Deno to canonical TypeScript transform which can be useful for
bundlers, use the following:

```ts
// docs: https://doc.deno.land/https/deno.land/x/dnt/transform.ts
import { transform } from "https://deno.land/x/dnt/transform.ts";

const outputFiles = await transform({
  entryPoint: "./mod.ts",
  shimPackageName: "deno-shim-package-name",
  keepExtensions: false, // transforms to not have extensions
});
```

## Rust API Example

When using TypeScript, the Rust API only transforms from Deno to canonical
TypeScript. You will need to use the TypeScript compiler to do the rest.

```rust
use std::path::PathBuf;

use deno_node_transform::ModuleSpecifier;
use deno_node_transform::transform;
use deno_node_transform::TransformOptions;

let output_files = transform(TransformOptions {
  entry_point: ModuleSpecifier::from_file_path(PathBuf::from("./mod.ts")).unwrap(),
  keep_extensions: false,
  loader: None, // use the default loader
}).await?;

for output_file in output_files {
  // use these properties on output_file
  output_file.file_path;
  output_file.file_text;
}
```
