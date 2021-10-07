# dnt - Deno to Node Transform

[![deno doc](https://doc.deno.land/badge.svg)](https://doc.deno.land/https/deno.land/x/dnt/mod.ts)

Prototype for a Deno to Node/canonical TypeScript transform.

Note: This is not completely working yet. Please don't use it as it will
probably drastically change.

## CLI Example

Create a configuration file in the deno-first repository:

```json
// ex. dnt.json
{
  "entryPoint": "mod.ts",
  "typeCheck": true,
  "outDir": "./npm",
  "package": {
    "name": "my-package",
    "description": "My package.",
    "author": "My Name",
    "license": "MIT",
    "repository": {
      "type": "git",
      "url": "git+https://github.com/dsherret/my-package.git"
    },
    "bugs": {
      "url": "https://github.com/dsherret/my-package/issues"
    }
  }
}
```

```bash
# run tool. This will output an npm package with cjs and mjs distributions bundling remote dependencies
deno run --allow-read=./ --allow-write=./npm --allow-net --no-check https://deno.land/x/dnt/cli.ts --config ./dnt.json --packageVersion 0.1.0

# go to output directory and publish
cd npm
npm publish
```

## JS API Example

To emit the Deno-first sources to code that can be consumed in Node.js, use the
`emit` function:

```ts
// docs: https://doc.deno.land/https/deno.land/x/dnt/mod.ts
import { emit } from "https://deno.land/x/dnt/mod.ts";

const emitResult = await emit({
  entryPoint: "./mod.ts",
  outDir: "./dist",
  typeCheck: false,
  shimPackage: {
    name: "deno.ns",
    version: "0.4.0",
  },
  package: {
    // package.json properties
    name: "my-package",
    version: "0.1.0",
    description: "My package.",
    license: "MIT",
  },
});
```

For only the Deno to canonical TypeScript transform which can be useful for
bundlers, use the following:

```ts
// docs: https://doc.deno.land/https/deno.land/x/dnt/transform.ts
import { transform } from "https://deno.land/x/dnt/transform.ts";

const outputResult = await transform({
  entryPoint: "./mod.ts",
  shimPackageName: "deno.ns",
});

// inspect outputResult.cjsFiles and outputResult.mjsFiles here
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
}).await?;

for output_file in output_result.cjs_files {
  // use these properties on output_file
  output_file.file_path;
  output_file.file_text;
}
```
