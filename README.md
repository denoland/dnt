# dnt - Deno to Node Transform

[![JSR](https://jsr.io/badges/@deno/dnt)](https://jsr.io/@deno/dnt)

Deno to npm package build tool.

## What does this do?

Takes a Deno module and creates an npm package for use in Node.js.

There are several steps done in a pipeline:

1. Transforms Deno code to Node including files found by `deno test`.
   - Rewrites module specifiers.
   - Injects [shims](https://github.com/denoland/node_deno_shims) for any `Deno`
     namespace or other global name usages as specified.
   - Rewrites [esm.sh](https://esm.sh/) specifiers to bare specifiers and
     includes these dependencies in a package.json.
   - When remote modules cannot be resolved to an npm package, it downloads them
     and rewrites specifiers to make them local.
   - Allows mapping any specifier to an npm package.
1. Type checks the output.
1. Emits ESM, CommonJS, and TypeScript declaration files along with a
   _package.json_ file.
1. Runs the final output in Node.js through a test runner calling all
   `Deno.test` calls.

## Setup

1. `deno add jsr:@deno/dnt`

1. Create a build script file:

   ```ts
   // ex. scripts/build_npm.ts
   import { build, emptyDir } from "@deno/dnt";

   await emptyDir("./npm");

   await build({
     entryPoints: ["./mod.ts"],
     outDir: "./npm",
     shims: {
       // see JS docs for overview and more options
       deno: true,
     },
     package: {
       // package.json properties
       name: "your-package",
       version: Deno.args[0],
       description: "Your package.",
       license: "MIT",
       repository: {
         type: "git",
         url: "git+https://github.com/username/repo.git",
       },
       bugs: {
         url: "https://github.com/username/repo/issues",
       },
     },
     postBuild() {
       // steps to run after building and before running the tests
       Deno.copyFileSync("LICENSE", "npm/LICENSE");
       Deno.copyFileSync("README.md", "npm/README.md");
     },
   });
   ```

1. Ignore the output directory with your source control if you desire (ex. add
   `npm/` to `.gitignore`).

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
[dnt] Type checking ESM...
[dnt] Emitting ESM package...
[dnt] Emitting script package...
[dnt] Running tests...

> test
> node test_runner.js

Running tests in ./script/mod.test.js...

test escapeWithinString ... ok
test escapeChar ... ok

Running tests in ./esm/mod.test.js...

test escapeWithinString ... ok
test escapeChar ... ok
[dnt] Complete!
```

## Docs

### Disabling Type Checking, Testing, Declaration Emit, or CommonJS/UMD Output

Use the following options to disable any one of these, which are enabled by
default:

```ts
await build({
  // ...etc...
  typeCheck: false,
  test: false,
  declaration: false,
  scriptModule: false,
});
```

### Type Checking Both ESM and Script Output

By default, only the ESM output will be type checked for performance reasons.
That said, it's recommended to type check both the ESM and the script (CJS/UMD)
output by setting `typeCheck` to `"both"`:

```ts
await build({
  // ...etc...
  typeCheck: "both",
});
```

### Ignoring Specific Type Checking Errors

Sometimes you may be getting a TypeScript error that is not helpful and you want
to ignore it. This is possible by using the `filterDiagnostic` option:

```ts
await build({
  // ...etc...
  filterDiagnostic(diagnostic) {
    if (
      diagnostic.file?.fileName.endsWith("fmt/colors.ts")
    ) {
      return false; // ignore all diagnostics in this file
    }
    // etc... more checks here
    return true;
  },
});
```

This is especially useful for ignoring type checking errors in remote
dependencies.

### Top Level Await

Top level await doesn't work in CommonJS/UMD and dnt will error if a top level
await is used and you are outputting CommonJS/UMD code. If you want to output a
CommonJS/UMD package then you'll have to restructure your code to not use any
top level awaits. Otherwise, set the `scriptModule` build option to `false`:

```ts
await build({
  // ...etc...
  scriptModule: false,
});
```

### Shims

dnt will shim the globals specified in the build options. For example, if you
specify the following build options:

```ts
await build({
  // ...etc...
  shims: {
    deno: true,
  },
});
```

Then write a statement like so...

```ts
Deno.readTextFileSync(...);
```

...dnt will create a shim file in the output, re-exporting the
[@deno/shim-deno](https://github.com/denoland/node_deno_shims) npm shim package
and change the Deno global to be used as a property of this object.

```ts
import * as dntShim from "./_dnt.shims.js";

dntShim.Deno.readTextFileSync(...);
```

#### Test-Only Shimming

If you want a shim to only be used in your test code as a dev dependency, then
specify `"dev"` for the option.

For example, to use the `Deno` namespace only for development and the
`setTimeout` and `setInterval` browser/Deno compatible shims in the distributed
code, you would do:

```ts
await build({
  // ...etc...
  shims: {
    deno: "dev",
    timers: true,
  },
});
```

#### Preventing Shimming

To prevent shimming in specific instances, add a `// dnt-shim-ignore` comment:

```ts
// dnt-shim-ignore
Deno.readTextFileSync(...);
```

...which will now output that code as-is.

#### Built-In Shims

Set any of these properties to `true` (distribution and test) or `"dev"` (test
only) to use them.

- `deno` - Shim the `Deno` namespace.
- `timers` - Shim the global `setTimeout` and `setInterval` functions with Deno
  and browser compatible versions.
- `prompts` - Shim the global `confirm`, `alert`, and `prompt` functions.
- `blob` - Shim the `Blob` global with the one from the `"buffer"` module.
- `crypto` - Shim the `crypto` global.
- `domException` - Shim the `DOMException` global using the "domexception"
  package (https://www.npmjs.com/package/domexception)
- `undici` - Shim `fetch`, `File`, `FormData`, `Headers`, `Request`, and
  `Response` by using the "undici" package
  (https://www.npmjs.com/package/undici).
- `weakRef` - Sham for the `WeakRef` global, which uses `globalThis.WeakRef`
  when it exists. The sham will throw at runtime when calling `deref()` and
  `WeakRef` doesn't globally exist, so this is only intended to help type check
  code that won't actually use it.
- `webSocket` - Shim `WebSocket` by using the
  [ws](https://www.npmjs.com/package/ws) package.

##### `Deno.test`-only shim

If you only want to shim `Deno.test` then provide the following:

```ts
await build({
  // ...etc...
  shims: {
    deno: {
      test: "dev",
    },
  },
});
```

This may be useful in Node v14 and below where the full deno shim doesn't always
work. See the section on Node v14 below for more details

#### Custom Shims (Advanced)

In addition to the pre-defined shim options, you may specify your own custom
packages to use to shim globals.

For example:

```ts
await build({
  scriptModule: false, // node-fetch 3+ only supports ESM
  // ...etc...
  shims: {
    custom: [{
      package: {
        name: "node-fetch",
        version: "~3.1.0",
      },
      globalNames: [{
        // for the `fetch` global...
        name: "fetch",
        // use the default export of node-fetch
        exportName: "default",
      }, {
        name: "RequestInit",
        typeOnly: true, // only used in type declarations
      }],
    }, {
      // this is what `blob: true` does internally
      module: "buffer", // uses node's "buffer" module
      globalNames: ["Blob"],
    }, {
      // this is what `domException: true` does internally
      package: {
        name: "domexception",
        version: "^4.0.0",
      },
      typesPackage: {
        name: "@types/domexception",
        version: "^4.0.0",
      },
      globalNames: [{
        name: "DOMException",
        exportName: "default",
      }],
    }],
    // shims to only use in the tests
    customDev: [{
      // this is what `timers: "dev"` does internally
      package: {
        name: "@deno/shim-timers",
        version: "~0.1.0",
      },
      globalNames: ["setTimeout", "setInterval"],
    }],
  },
});
```

#### Local and Remote Shims

Custom shims can also refer to local or remote modules:

```ts
await build({
  // ...etc...
  shims: {
    custom: [{
      module: "./my-custom-fetch-implementation.ts",
      globalNames: ["fetch"],
    }, {
      module: "https://deno.land/x/some_remote_shim_module/mod.ts",
      globalNames: ["setTimeout"],
    }],
  },
});
```

Where `my-custom-fetch-implementation.ts` contains:

```ts
export function fetch(/* etc... */) {
  // etc...
}
```

This is useful in situations where you want to implement your own shim.

### Specifier to npm Package Mappings

In most cases, dnt won't know about an npm package being available for one of
your dependencies and will download remote modules to include in your package.
There are scenarios though where an npm package may exist and you want to use it
instead. This can be done by providing a specifier to npm package mapping.

For example:

```ts
await build({
  // ...etc...
  mappings: {
    "https://deno.land/x/code_block_writer@11.0.0/mod.ts": {
      name: "code-block-writer",
      version: "^11.0.0",
      // optionally specify if this should be a peer dependency
      peerDependency: false,
    },
  },
});
```

This will:

1. Change all `"https://deno.land/x/code_block_writer@11.0.0/mod.ts"` specifiers
   to `"code-block-writer"`
2. Add a package.json dependency for `"code-block-writer": "^11.0.0"`.

Note that dnt will error if you specify a mapping and it is not found in the
code. This is done to prevent the scenario where a remote specifier's version is
bumped and the mapping isn't updated.

#### Mapping specifier to npm package subpath

Say an npm package called `example` had a subpath at `sub_path.js` and you
wanted to map `https://deno.land/x/example@0.1.0/sub_path.ts` to that subpath.
To specify this, you would do the following:

```ts
await build({
  // ...etc...
  mappings: {
    "https://deno.land/x/example@0.1.0/sub_path.ts": {
      name: "example",
      version: "^0.1.0",
      subPath: "sub_path.js", // note this
    },
  },
});
```

This would cause the following:

```ts
import * as mod from "https://deno.land/x/example@0.1.0/sub_path.ts";
```

...to go to...

```ts
import * as mod from "example/sub_path.js";
```

...with a dependency on `"example": "^0.1.0"`.

### Multiple Entry Points

To do this, specify multiple entry points like so (ex. an entry point at `.` and
another at `./internal`):

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
  "main": "./script/mod.js",
  "module": "./esm/mod.js",
  "types": "./types/mod.d.ts",
  "exports": {
    ".": {
      "import": {
        "types": "./types/mod.d.ts",
        "default": "./esm/mod.js"
      },
      "require": {
        "types": "./types/mod.d.ts",
        "default": "./script/mod.js"
      }
    },
    "./internal": {
      "import": {
        "types": "./types/internal.d.ts",
        "default": "./esm/internal.js"
      },
      "require": {
        "types": "./types/internal.d.ts",
        "default": "./script/internal.js"
      }
    }
  }
}
```

Now these entry points could be imported like
`import * as main from "your-package"` and
`import * as internal from "your-package/internal";`.

### Bin/CLI Packages

To publish an npm
[bin package](https://docs.npmjs.com/cli/v7/configuring-npm/package-json#bin)
similar to `deno install`, add a `kind: "bin"` entry point:

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

This will add a `"bin"` entry to the package.json and add `#!/usr/bin/env node`
to the top of the specified entry point.

### Node and Deno Specific Code

You may find yourself in a scenario where you want to run certain code based on
whether someone is in Deno or if someone is in Node and feature testing is not
possible. For example, say you want to run the `deno` executable when the code
is running in Deno and the `node` executable when it's running in Node.

#### `which_runtime`

One option to handle this, is to use the
[`which_runtime`](https://deno.land/x/which_runtime) deno.land/x module which
provides some exports saying if the code is running in Deno or Node.

#### Node and Deno Specific Modules

Another option is to create node and deno specific modules. This can be done by
specifying a mapping to a module:

```ts
await build({
  // ...etc...
  mappings: {
    "./file.deno.ts": "./file.node.ts",
  },
});
```

Then within the file, use `// dnt-shim-ignore` directives to disable shimming if
you desire.

A mapped module should be written similar to how you write Deno code (ex. use
extensions on imports), except you can also import built-in node modules such as
`import fs from "fs";` (just remember to include an `@types/node` dev dependency
under the `package.devDependencies` object when calling the `build` function, if
necessary).

### Pre & Post Build Steps

Since the file you're calling is a script, simply add statements before and
after the `await build({ ... })` statement:

```ts
import { build, emptyDir } from "https://deno.land/x/dnt/mod.ts";

// run pre-build steps here
await emptyDir("./npm");

// build
await build({
  // ...etc..
});

// run post-build steps here
await Deno.copyFile("LICENSE", "npm/LICENSE");
await Deno.copyFile("README.md", "npm/README.md");
```

### Including Test Data Files

Your Deno tests might rely on test data files. One way of handling this is to
copy these files to be in the output directory at the same relative path your
Deno tests run with.

For example:

```ts
import { copy } from "https://deno.land/std@x.x.x/fs/mod.ts";

await Deno.remove("npm", { recursive: true }).catch((_) => {});
await copy("testdata", "npm/esm/testdata", { overwrite: true });
await copy("testdata", "npm/script/testdata", { overwrite: true });

await build({
  // ...etc...
});

// ensure the test data is ignored in the `.npmignore` file
// so it doesn't get published with your npm package
await Deno.writeTextFile(
  "npm/.npmignore",
  "esm/testdata/\nscript/testdata/\n",
  { append: true },
);
```

Alternatively, you could also use the
[`which_runtime`](https://deno.land/x/which_runtime) module and use a different
directory path when the tests are running in Node. This is probably more ideal
if you have a lot of test data.

### Test File Matching

By default, dnt uses the same search [pattern](https://deno.land/manual/testing)
that `deno test` uses to find test files. To override this, provide a
`testPattern` and/or `rootTestDir` option:

```ts
await build({
  // ...etc...
  testPattern: "**/*.test.{ts,tsx,js,mjs,jsx}",
  // and/or provide a directory to start searching for test
  // files from, which defaults to the current working directory
  rootTestDir: "./tests",
});
```

### deno.json Support

Starting in dnt 0.42, the deno.json is auto-discovered. A config file can be
explicitly specified by the `configFile` key:

```ts
await build({
  // ...etc...
  configFile: import.meta.resolve("../deno.json"),
});
```

### GitHub Actions - Npm Publish on Tag

1. Ensure your build script accepts a version as a CLI argument and sets that in
   the package.json object. For example:

   ```ts
   await build({
     // ...etc...
     package: {
       version: Deno.args[0],
       // ...etc...
     },
   });
   ```

   Note: You may wish to remove the leading `v` in the tag name if it exists
   (ex. `Deno.args[0]?.replace(/^v/, "")`)

1. In your npm settings, create an _automation_ access token (see
   [Creating and Viewing Access Tokens](https://docs.npmjs.com/creating-and-viewing-access-tokens)).

1. In your GitHub repo or organization, add a secret for `NPM_TOKEN` with the
   value created in the previous step (see
   [Creating Encrypted Secrets for a Repository](https://docs.github.com/en/actions/security-guides/encrypted-secrets#creating-encrypted-secrets-for-a-repository)).

1. In your GitHub Actions workflow, get the tag name, setup node, run your build
   script, then publish to npm.

   ```yml
   # ...setup deno and run `deno test` here as you normally would...

   - name: Get tag version
     if: startsWith(github.ref, 'refs/tags/')
     id: get_tag_version
     run: echo TAG_VERSION=${GITHUB_REF/refs\/tags\//} >> $GITHUB_OUTPUT
   - uses: actions/setup-node@v3
     with:
       node-version: "18.x"
       registry-url: "https://registry.npmjs.org"
   - name: npm build
     run: deno run -A ./scripts/build_npm.ts ${{steps.get_tag_version.outputs.TAG_VERSION}}
   - name: npm publish
     if: startsWith(github.ref, 'refs/tags/')
     env:
       NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
     run: cd npm && npm publish
   ```

   Note that the build script always runs even when not publishing. This is to
   ensure your build and tests pass on each commit.

1. Ensure the workflow will run on tag creation. For example, see
   [Trigger GitHub Action Only on New Tags](https://stackoverflow.com/q/61891328/188246)).

### Using Another Package Manager

You may want to use another Node.js package manager instead of npm, such as Yarn
or pnpm. To do this, override the `packageManager` option in the build options.

For example:

```ts
await build({
  // ...etc...
  packageManager: "yarn", // or "pnpm"
});
```

You can even specify an absolute path to the executable file of the package
manager:

```ts
await build({
  // ...etc...
  packageManager: "/usr/bin/pnpm",
});
```

### DOM Types

If you wish to compile with DOM types for type checking, you may specify a "dom"
lib compiler option when building:

```ts
await build({
  // ...etc...
  compilerOptions: {
    lib: ["ES2021", "DOM"],
  },
});
```

### Node v14 and Below

dnt should be able to target old versions of Node by specifying a
`{ compilerOption: { target: ... }}` value in the build options (see
[Node Target Mapping](https://github.com/microsoft/TypeScript/wiki/Node-Target-Mapping)
for what target maps to what Node version). A problem though is that certain
shims might not work in old versions of Node.

If wanting to target a version of Node v14 and below, its recommend to use the
`Deno.test`-only shim (described above) and then making use of the "mappings"
feature to write Node-only files where you can handle differences.
Alternatively, see if changes to the shim libraries might make it run on old
versions of Node. Unfortunately, certain features are impossible or infeasible
to get working.

See [this thread](https://github.com/denoland/node_deno_shims/issues/15) in
node_deno_shims for more details.

## JS API Example

For only the Deno to canonical TypeScript transform which may be useful for
bundlers, use the following:

```ts
// docs: https://doc.deno.land/https/deno.land/x/dnt/transform.ts
import { transform } from "https://deno.land/x/dnt/transform.ts";

const outputResult = await transform({
  entryPoints: ["./mod.ts"],
  testEntryPoints: ["./mod.test.ts"],
  shims: [],
  testShims: [],
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
  shims: vec![],
  test_shims: vec![],
  loader: None, // use the default loader
  specifier_mappings: None,
}).await?;
```
