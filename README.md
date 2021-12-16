# dnt - Deno to Node Transform

[![deno doc](https://doc.deno.land/badge.svg)](https://doc.deno.land/https/deno.land/x/dnt/mod.ts)

Deno to npm package build tool.

This tool is under active early development and hasn't been tested in a lot of scenarios. Examine its output thoroughly before publishing. If you encounter any problems or challenges, please open an [issue](https://github.com/denoland/dnt/issues) to help us improve it.

## What does this do?

Takes a Deno module and creates an npm package for use in Node.js.

There are several steps done in a pipeline:

1. Transforms Deno code to Node/canonical TypeScript including files found by `deno test`.
   - Rewrites module specifiers.
   - Injects a [Deno shim](https://github.com/denoland/deno.ns) for any `Deno` namespace or other global name usages.
   - Rewrites [Skypack](https://www.skypack.dev/) and [esm.sh](https://esm.sh/) specifiers to bare specifiers and includes these dependencies in a package.json.
   - When remote modules cannot be resolved to an npm package, it downloads them and rewrites specifiers to make them local.
   - Allows mapping any specifier to an npm package.
1. Type checks the output.
1. Emits ESM, CommonJS, and TypeScript declaration files along with a _package.json_ file.
1. Runs the final output in Node.js through a test runner calling all `Deno.test` calls.

## Setup

1. Create a build script file:

   ```ts
   // ex. scripts/build_npm.ts
   import { build } from "https://deno.land/x/dnt/mod.ts";

   await build({
     entryPoints: ["./mod.ts"],
     outDir: "./npm",
     package: {
       // package.json properties
       name: "your-package",
       version: Deno.args[0],
       description: "Your package.",
       license: "MIT",
       repository: {
         type: "git",
         url: "git+https://github.com/username/package.git",
       },
       bugs: {
         url: "https://github.com/username/package/issues",
       },
     },
   });

   // post build steps
   Deno.copyFileSync("LICENSE", "npm/LICENSE");
   Deno.copyFileSync("README.md", "npm/README.md");
   ```

1. Ignore the output directory with your source control if you desire (ex. add `npm/` to `.gitignore`).

1. Run it and `npm publish`:

   ```bash
   # run script
   deno run -A scripts/build_npm.ts 0.1.0

   # go to output directory and publish
   cd npm
   npm publish
   ```

### Example Build Logs

```
[dnt] Transforming...
[dnt] Running npm install...
[dnt] Building project...
[dnt] Type checking...
[dnt] Emitting declaration files...
[dnt] Emitting ESM package...
[dnt] Emitting CommonJS package...
[dnt] Running tests...

> test
> node test_runner.js

Running tests in ./umd/mod.test.js...

test escapeWithinString ... ok
test escapeChar ... ok

Running tests in ./esm/mod.test.js...

test escapeWithinString ... ok
test escapeChar ... ok
[dnt] Complete!
```

## Docs

### Disabling Type Checking, Testing, Declaration Emit, or CommonJS Output

Use the following options to disable any one of these, which are enabled by default:

```ts
await build({
  // ...etc...
  typeCheck: false,
  test: false,
  declaration: false,
  cjs: false,
});
```

### Top Level Await

Top level await doesn't work in CommonJS and dnt will error if a top level await is used and you are outputting CommonJS code. If you want to output a CommonJS package then you'll have to restructure your code to not use any top level awaits. Otherwise, set the `cjs` build option to `false`:

```ts
await build({
  // ...etc...
  cjs: false,
});
```

### Specifier to Npm Package Mappings

In most cases, dnt won't know about an npm package being available for one of your dependencies and will download remote modules to include in your package. There are scenarios though where an npm package may exist and you want to use it instead. This can be done by providing a specifier to npm package mapping.

For example:

```ts
await build({
  // ...etc...
  mappings: {
    "https://deno.land/x/code_block_writer@11.0.0/mod.ts": {
      name: "code-block-writer",
      version: "^11.0.0",
    },
  },
});
```

This will:

1. Change all `"https://deno.land/x/code_block_writer@11.0.0/mod.ts"` specifiers to `"code-block-writer"`
2. Add a package.json dependency for `"code-block-writer": "^11.0.0"`.

Note that dnt will error if you specify a mapping and it is not found in the code. This is done to prevent the scenario where a remote specifier's version is bumped and the mapping isn't updated.

### Multiple Entry Points

To do this, specify multiple entry points like so (ex. an entry point at `.` and another at `./internal`):

```ts
await build({
  entryPoints: ["mod.ts", {
    name: "./internal",
    path: "internal.ts",
  }],
  // ...etc...
});
```

This will create a package.json with these as exports:

```jsonc
{
  "name": "your-package",
  // etc...
  "main": "./umd/mod.js",
  "module": "./esm/mod.js",
  "types": "./types/mod.d.ts",
  "exports": {
    ".": {
      "import": "./esm/mod.js",
      "require": "./umd/mod.js",
      "types": "./types/mod.d.ts"
    },
    "./internal": {
      "import": "./esm/internal.js",
      "require": "./umd/internal.js",
      "types": "./types/internal.d.ts"
    }
  }
}
```

Now these entry points could be imported like `import * as main from "your-package"` and `import * as internal from "your-package/internal";`.

### Bin/CLI Packages

To publish an npm [bin package](https://docs.npmjs.com/cli/v7/configuring-npm/package-json#bin) similar to `deno install`, add a `kind: "bin"` entry point:

```ts
await build({
  entryPoints: [{
    kind: "bin",
    name: "my_binary", // command name
    path: "./cli.ts",
  }],
  // ...etc...
});
```

This will add a `"bin"` entry to the package.json and add `#!/usr/bin/env node` to the top of the specified entry point.

### Preventing Shimming

Dnt shims calls to Deno globals like so...

```ts
Deno.readTextFileSync(...);
```

...by adding an import statement to a [shim library](https://github.com/denoland/deno.ns) and changing the Deno global to reference to point at it.

```ts
import * as denoShim from "deno.ns";

denoShim.Deno.readTextFileSync(...);
```

Maybe there are scenarios where you don't want that to happen though. To prevent it, add a `// deno-shim-ignore` comment:

```ts
// deno-shim-ignore
Deno.readTextFileSync(...);
```

...which will now output that code as-is.

### Node and Deno Specific Code

You may find yourself in a scenario where you want to run certain code based on whether someone is in Deno or if someone is in Node and feature testing is not possible. For example, say you want to run the `deno` executable when the code is running in Deno and the `node` executable when it's running in Node.

#### `which_runtime`

One option to handle this, is to use the [`which_runtime`](https://deno.land/x/which_runtime) deno.land/x module which provides some exports saying if the code is running in Deno or Node.

#### Node and Deno Specific Modules

Another option is to create node and deno specific modules. This can be done by specifying a redirect:

```ts
await build({
  // ...etc...
  redirects: {
    "./file.deno.ts": "./file.node.ts",
  },
});
```

Then within the file, use `// deno-shim-ignore` directives to disable shimming if you desire.

### Pre & Post Build Steps

Since the file you're calling is a script, simply add statements before and after the `await build({ ... })` statement:

```ts
// run pre-build steps here

// ex. maybe consider deleting the output directory before build
await Deno.remove("npm", { recursive: true }).catch((_) => {});

await build({
  // ...etc..
});

// run post-build steps here
await Deno.copyFile("LICENSE", "npm/LICENSE");
await Deno.copyFile("README.md", "npm/README.md");
```

### Including Test Data Files

Your Deno tests might rely on test data files. One way of handling this is to copy these files to be in the output directory at the same relative path your Deno tests run with.

For example:

```ts
import { copy } from "https://deno.land/std@x.x.x/fs/mod.ts";

await Deno.remove("npm", { recursive: true }).catch((_) => {});
await copy("testdata", "npm/esm/testdata", { overwrite: true });
await copy("testdata", "npm/umd/testdata", { overwrite: true });

await build({
  // ...etc...
});

// ensure the test data is ignored in the `.npmignore` file
// so it doesn't get published with your npm package
await Deno.writeTextFile(
  "npm/.npmignore",
  "esm/testdata/\numd/testdata/\n",
  { append: true },
);
```

Alternatively, you could also use the [`which_runtime`](https://deno.land/x/which_runtime@0.1.0) module and use a different directory path when the tests are running in Node. This is probably more ideal if you have a lot of test data.

### Test File Matching

By default, dnt uses the same search [pattern](https://deno.land/manual/testing) that `deno test` uses to find test files. To override this, provide a `testPattern` and/or `rootTestDir` option:

```ts
await build({
  // ...etc...
  testPattern: "**/*.test.{ts,tsx,js,mjs,jsx}",
  // and/or provide a directory to start searching for test
  // files from, which defaults to the current working directory
  rootTestDir: "./tests",
});
```

### GitHub Actions - Npm Publish on Tag

1. Ensure your build script accepts a version as a CLI argument and sets that in the package.json object. For example:

   ```ts
   await build({
     // ...etc...
     package: {
       version: Deno.args[0],
       // ...etc...
     },
   });
   ```

   Note: You may wish to remove the leading `v` in the tag name if it exists (ex. `Deno.args[0]?.replace(/^v/, "")`)

1. In your npm settings, create an _automation_ access token (see [Creating and Viewing Access Tokens](https://docs.npmjs.com/creating-and-viewing-access-tokens)).

1. In your GitHub repo or organization, add a secret for `NPM_TOKEN` with the value created in the previous step (see [Creating Encrypted Secrets for a Repository](https://docs.github.com/en/actions/security-guides/encrypted-secrets#creating-encrypted-secrets-for-a-repository)).

1. In your GitHub Actions workflow, get the tag name, setup node, run your build script, then publish to npm.

   ```yml
   # ...setup deno and run `deno test` here as you normally would...

   - name: Get tag version
     if: startsWith(github.ref, 'refs/tags/')
     id: get_tag_version
     run: echo ::set-output name=TAG_VERSION::${GITHUB_REF/refs\/tags\//}
   - uses: actions/setup-node@v2
     with:
       node-version: '16.x'
       registry-url: 'https://registry.npmjs.org'
   - name: npm build
     run: deno run -A ./scripts/build_npm.ts ${{steps.get_tag_version.outputs.TAG_VERSION}}
   - name: npm publish
     if: startsWith(github.ref, 'refs/tags/')
     env:
       NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
     run: |
       cd npm
       npm publish
   ```

   Note that the build script always runs even when not publishing. This is to ensure your build and tests pass on each commit.

1. Ensure the workflow will run on tag creation. For example, see [Trigger GitHub Action Only on New Tags](https://stackoverflow.com/q/61891328/188246)).

## JS API Example

For only the Deno to canonical TypeScript transform which may be useful for bundlers, use the following:

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
