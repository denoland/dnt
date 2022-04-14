# dnt - Deno 代码转换为 Node 代码的构建工具

[![deno doc](https://doc.deno.land/badge.svg)](https://doc.deno.land/https/deno.land/x/dnt/mod.ts)

**中文** | [English](./README.md)

转换 Deno 为 npm package 的构建工具。

目前这个工具还处于早期开发阶段，仍然需要大量场景的测试。在发布前，还需要仔细检查构建的输出结果。如果你遇到任何问题或者挑战，请提交一个 [issue](https://github.com/denoland/dnt/issues) 来帮助我们改善这个工具。

## dnt 有什么用？

Dnt 可以把一个 Deno 模块转换为可以在 Node.js 下使用的 npm 包。

在一个流水线里，实际会发现下面的步骤:

1. 把 Deno 的代码包括 `deno test` 发现的代码转换为 Node/标准的 TypeScript 代码。
   - 重写模块标识符。
   - 注入 [shims](https://github.com/denoland/node_deno_shims) 到任何 `Deno` 的名字空间或者配置的其他全局变量。
   - 把 [Skypack](https://www.skypack.dev/) 和 [esm.sh](https://esm.sh/) 标识符改为裸标识符，然后把这些依赖注入 package.json。
   - 当远程依赖无法转换为一个 npm 包时，dnt 会下载他们，并且改写他们的标识符，并且把这些依赖作为本地依赖。
   - 允许映射任何标识符到一个 npm 包。
2. 对输出进行类型检查。
3. 发射 ESM，CommonJS，和 TypeScript 类型声明文件在 package.json 里。
4. 使用 `Deno.test` 对最终的输入结果在 Node.js 里进行测试校验。

## 安装

1. 创建一个构建脚本文件:

   ```ts
   // ex. scripts/build_npm.ts
   import { build, emptyDir } from 'https://deno.land/x/dnt/mod.ts'

   await emptyDir('./npm')

   await build({
     entryPoints: ['./mod.ts'],
     outDir: './npm',
     shims: {
       // 可以在 JS 文档里查看更详细的配置属性
       deno: true,
     },
     package: {
       // package.json 属性
       name: 'your-package',
       version: Deno.args[0],
       description: 'Your package.',
       license: 'MIT',
       repository: {
         type: 'git',
         url: 'git+https://github.com/username/repo.git',
       },
       bugs: {
         url: 'https://github.com/username/repo/issues',
       },
     },
   })

   // 构建后置步骤
   Deno.copyFileSync('LICENSE', 'npm/LICENSE')
   Deno.copyFileSync('README.md', 'npm/README.md')
   ```

2. 如果你需要，可以忽略输出文件夹(例如，在 `.gitignore` 增加 `npm/`)。
3. 运行脚本，并且运行 `npm publish`:

   ```bash
   # 运行脚本
   deno run -A scripts/build_npm.ts 0.1.0

   # 切换到输出文件夹并且发布
   cd npm
   npm publish
   ```

### 构建日志样例

```
[dnt] Transforming...
[dnt] Running npm install...
[dnt] Building project...
[dnt] Type checking...
[dnt] Emitting declaration files...
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

## 文档

### 关闭类型检查，测试，发射类型文件，或者 CommonJS/UMD 输出

使用下面的选项来关闭上面的设置，默认都会添加:

```ts
await build({
  // ...etc...
  typeCheck: false,
  test: false,
  declaration: false,
  scriptModule: false,
})
```

### Top Level Await

因为 Commonjs/UMD 不支持 Top level await，所以如果你输出 Commonjs/UMD，dnt 会给你一个报错。如果你想输出 Commonjs/UMD，你就需要调整你的代码，不要使用 Top level await。否则，你需要把构建选项 `scriptModule` 改为 `false`:

```ts
await build({
  // ...etc...
  scriptModule: false,
})
```

### Shims

dnt 提供了选项来支持全局标识符的 shim。例如，如果你制定了下列的构建选项:

```ts
await build({
  // ...etc...
  shims: {
    deno: true,
  },
})
```

并且你的代码里，有这样的语句:

```ts
Deno.readTextFileSync(...);
```

...dnt 将会在输出给你创建一个 shim 文件，引入 [@deno/shim-deno](https://github.com/denoland/node_deno_shims)，并且把你的代码改为:

```ts
import * as dntShim from "./_dnt.shims.js";

dntShim.Deno.readTextFileSync(...);
```

#### Test-Only Shimming

如果你想只在你的测试代码使用作为开发依赖的 shim，可以指定选项为 `"dev"`。

例如，只在开发时使用 `Deno` 名字空间和 `setTimeout` 和`setInterval` 等浏览器/Deno 兼容的 shims 在发布环境，你可以这样做:

```ts
await build({
  // ...etc...
  shims: {
    deno: 'dev',
    timers: true,
  },
})
```

#### 阻止 Shimming

如果你想在特定的语句，阻止 shimming，可以添加注释 `// dnt-shim-ignore`:

```ts
// dnt-shim-ignore
Deno.readTextFileSync(...);
```

...这样做，输出的代码就是原来写的代码。

#### 内置 Shims

将选项置为 `true` (在发布环境和测试时)或者 `"dev"`(在测试时)来使用这些 shims。

- `deno` - `Deno` 名字空间的 Shim。
- `timers` - 全局 `setTimeout` 和 `setInterval` 的 Deno 与浏览器兼容版本的 Shim。
- `prompts` - 全局 `confirm`, `alert`, 和 `prompt` 函数的 Shim。
- `blob` - `"buffer"` 模块的 `Blob` 的 Shim。
- `crypto` - `crypto` 的全局 Shim.
- `domException` - 使用 "domexception" 包(https://www.npmjs.com/package/domexception) 的 `DOMException` 的 Shim。
- `undici` - 通过 "undici" 包 (https://www.npmjs.com/package/undici) 来对 `fetch`, `File`, `FormData`, `Headers`, `Request`, 和 `Response` 进行 Shim。
- `weakRef` - 通过存在 `globalThis.WeakRef` 时使用 `globalThis.WeakRef` 来对 `WeakRef` 进行 Shim. 这个 shim 在 `deref()` 和 `WeakRef` 不存在时会报错。所以这个 shim 只能确保代码类型检查，但是并不能实际使用他们。

##### `Deno.test`-only shim

如果你只想对 `Deno.test` 进行 shim，你可以使用下面这个设置:

```ts
await build({
  // ...etc...
  shims: {
    deno: {
      test: 'dev',
    },
  },
})
```

这对于 Node v14 以及更低的版本可能更有用，因为目前提供的 shim 在这些版本并不能完全工作。如果你对这部分感兴趣，可以继续看下面关于 Node v14 的 shim 的细节。

#### 自定义 Shims (高级)

为了增加预定义的 shim 选项，你可能想增加自己的包来进行 shim。

例如:

```ts
await build({
  scriptModule: false, // node-fetch 3+ 只支持 ESM
  // ...etc...
  shims: {
    custom: [
      {
        package: {
          name: 'node-fetch',
          version: '~3.1.0',
        },
        globalNames: [
          {
            // 对于全局 `fetch` ...
            name: 'fetch',
            // 对于 node-fetch 使用 default 导出
            exportName: 'default',
          },
          {
            name: 'RequestInit',
            typeOnly: true, // 只使用类型声明文件
          },
        ],
      },
      {
        // 这个选项实际是 `blob: true` 内部的实际选项
        module: 'buffer', // 使用 node 的 "buffer" 模块
        globalNames: ['Blob'],
      },
      {
        // 这是 `domException: true` 内部的详细选项
        package: {
          name: 'domexception',
          version: '^4.0.0',
        },
        typesPackage: {
          name: '@types/domexception',
          version: '^2.0.1',
        },
        globalNames: [
          {
            name: 'DOMException',
            exportName: 'default',
          },
        ],
      },
    ],
    // 只对测试进行 shim
    customDev: [
      {
        // 这是 `timers: "dev"` 内部的详细选项
        package: {
          name: '@deno/shim-timers',
          version: '~0.1.0',
        },
        globalNames: ['setTimeout', 'setInterval'],
      },
    ],
  },
})
```

#### 本地和远程 Shims

自定义 shim 也可以指本地或者远程的模块:

```ts
await build({
  // ...etc...
  shims: {
    custom: [
      {
        module: './my-custom-fetch-implementation.ts',
        globalNames: ['fetch'],
      },
      {
        module: 'https://deno.land/x/some_remote_shim_module/mod.ts',
        globalNames: ['setTimeout'],
      },
    ],
  },
})
```

这里 `my-custom-fetch-implementation.ts` 包含了:

```ts
export function fetch(/* etc... */) {
  // etc...
}
```

这对于你实现自己的 shim 应该是有帮助的.

### Npm 包映射标识符

在绝大部分场景里，dnt 不知道你引用的依赖的实际 npm 包是什么，而是把远程依赖下载到本地，然后引入到你的项目里。这里有一个场景是，你知道 npm 包在哪里存在，并且你想用这个包。可以通过在构建配置中增加标识符映射 npm 包。

例如:

```ts
await build({
  // ...etc...
  mappings: {
    'https://deno.land/x/code_block_writer@11.0.0/mod.ts': {
      name: 'code-block-writer',
      version: '^11.0.0',
    },
  },
})
```

这么配置，会做一下工作:

1. 把所有 `"https://deno.land/x/code_block_writer@11.0.0/mod.ts"` 标识符改为`"code-block-writer"`
2. 在 package.json 增加依赖 `"code-block-writer": "^11.0.0"`.

如果你标识了一个映射，但是在你的代码中没有找到，dnt 会报错。这么做是为了防止远程标识符版本变了，而映射并没有更新。

#### 映射标识符到 npm 包的子路径

如果一个 npm 包叫 `example`，并且有一个子路径在 `sub_path.js`，并且你想把 `https://deno.land/x/example@0.1.0/sub_path.ts` 映射为这个子路径。为了实现这个，你需要设置:

```ts
await build({
  // ...etc...
  mappings: {
    'https://deno.land/x/example@0.1.0/sub_path.ts': {
      name: 'example',
      version: '^0.1.0',
      subPath: 'sub_path.js', // note this
    },
  },
})
```

这么做会把下面的代码:

```ts
import * as mod from 'https://deno.land/x/example@0.1.0/sub_path.ts'
```

...变为...

```ts
import * as mod from 'example/sub_path.js'
```

...并且增加一个依赖 `"example": "^0.1.0"`.

### 多进入点项目

可以通过下面这个设置来实现(例如，一个进入点在 `.`, 另一个在 `./internal`)

```ts
await build({
  entryPoints: [
    'mod.ts',
    {
      name: './internal',
      path: 'internal.ts',
    },
  ],
  // ...etc...
})
```

这么做会创建一个有多输出的 package.json:

```jsonc
{
  "name": "your-package",
  // etc...
  "main": "./script/mod.js",
  "module": "./esm/mod.js",
  "types": "./types/mod.d.ts",
  "exports": {
    ".": {
      "import": "./esm/mod.js",
      "require": "./script/mod.js",
      "types": "./types/mod.d.ts"
    },
    "./internal": {
      "import": "./esm/internal.js",
      "require": "./script/internal.js",
      "types": "./types/internal.d.ts"
    }
  }
}
```

现在这些进入点可以通过以下语句来引入: `import * as main from "your-package"` 和 `import * as internal from "your-package/internal";`。

### Bin/CLI 包

发布一个 [bin package](https://docs.npmjs.com/cli/v7/configuring-npm/package-json#bin) 和 `deno install` 类似, 增加一个 `kind: "bin"` 进入点:

```ts
await build({
  entryPoints: [
    {
      kind: 'bin',
      name: 'my_binary', // command name
      path: './cli.ts',
    },
  ],
  // ...etc...
})
```

这会给 package.json 增加一个 `"bin"` 的进入点。并且在进入点文件的头部增加 `#!/usr/bin/env node`。

### Node 和 Deno 指定的代码

你可能会发现这样一个场景，一些代码跑在 Deno 里，一些代码跑在 Node 里，然后进行一些功能的测试。例如，比如你想要让一些代码跑在 `deno` 执行程序里，一些代码跑在 `node` 执行程序里。

#### `which_runtime`

一种处理这种场景的选择是 使用 [`which_runtime`](https://deno.land/x/which_runtime) deno.land/x 模块，这个模块提供了一些方法，可以让代码跑在 Deno 或者 Node 里。

#### Node 和 Deno 指定的模块

另一种创建 node 和 deno 指定模块的方法是一个映射模块的选项:

```ts
await build({
  // ...etc...
  mappings: {
    './file.deno.ts': './file.node.ts',
  },
})
```

然后在文件内部, 使用 `// dnt-shim-ignore` 指令来消除 shim。

一个映射为 deno 的模块书写代码和你写其他 Deno 代码是一样(例如，import 需要加后缀名)，额外的区别是，你可以通过 `import fs from "fs";` 来引入 node 模块。(注意要添加 `@types/node` 开发依赖)。

### 构建前步骤 & 构建后步骤

因为你调用的文件是一个脚本，所以你只要在 `await build({ ... })` 前或者后增加你想增加的语句:

```ts
// 运行构建前步骤

// 例如. 在构建前把输出目录的内容都删除
await Deno.remove('npm', { recursive: true }).catch((_) => {})

await build({
  // ...etc..
})

// 运行构建后步骤
await Deno.copyFile('LICENSE', 'npm/LICENSE')
await Deno.copyFile('README.md', 'npm/README.md')
```

### 引入测试数据文件

你的 Deno 测试可能需要以来一些测试数据文件。一种方式是把这些文件拷贝到输出目录中。这样之前的相对目录都是正确的。

例如:

```ts
import { copy } from 'https://deno.land/std@x.x.x/fs/mod.ts'

await Deno.remove('npm', { recursive: true }).catch((_) => {})
await copy('testdata', 'npm/esm/testdata', { overwrite: true })
await copy('testdata', 'npm/script/testdata', { overwrite: true })

await build({
  // ...etc...
})

// 确保测试数据文件在 `.npmignore` 内
// 这样最后发布的包里不含有这部分文件
await Deno.writeTextFile(
  'npm/.npmignore',
  'esm/testdata/\nscript/testdata/\n',
  { append: true }
)
```

你也可以使用 [`which_runtime`](https://deno.land/x/which_runtime) 模块来让最后执行的 Node 需要的测试文件在一个特定的目录。这对于你有大量测试数据的时候，可能更有帮助。

### 测试文件匹配

dnt 默认使用和 `deno test` 一样的匹配模块 [pattern](https://deno.land/manual/testing) 来寻找测试文件。
你可以通过提供 `testPattern` 和 `rootTestDir` 选项来覆盖这个设置:

```ts
await build({
  // ...etc...
  testPattern: '**/*.test.{ts,tsx,js,mjs,jsx}',
  // 提供一个基准文件目录来搜索测试代码文件
  // 默认是当前的工作目录
  rootTestDir: './tests',
})
```

### GitHub Actions - 使用标签来发布 Npm

1. 确保你的构建脚本可以通过 CLI 命令来获取 package.json 的版本。例如：

   ```ts
   await build({
     // ...etc...
     package: {
       version: Deno.args[0],
       // ...etc...
     },
   })
   ```

   注意: 你可能需要通过代码来去掉标签的`v` 字段(例如. `Deno.args[0]?.replace(/^v/, "")`)

2. 在你的 npm 设置中，创建一个自动获取密钥(详细请参考 [Creating and Viewing Access Tokens](https://docs.npmjs.com/creating-and-viewing-access-tokens))

3. 在你的 GitHub 项目或者组织，增加 `NPM_TOKEN` 的密钥(详细请参考 [Creating and Viewing Access Tokens](https://docs.npmjs.com/creating-and-viewing-access-tokens))。

4. 在你的 GitHub Actions 工作流里，获得标签名称，安装节点，执行你的构建脚本，然后发布到 npm。

   ```yml
   # ...安装 deno 以及和你经常做的一样运行 `deno test` ...

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
     run: cd npm && npm publish
   ```

   注意构建脚本在不进行发布时也会经常运行。这样能保证你的构建和测试能在每一次提交时都是正常的。

5. 确保你的工作流可以在每个标签创建时进行构建。例如，可以参考[Trigger GitHub Action Only on New Tags](https://stackoverflow.com/q/61891328/188246)。

### 使用其他的包管理工具

你可能想用其他的包管理工具，而不是 npm，例如 Yarn 或者 pnpm。通过覆盖 `packageManager` 选项来实现这一目标。

例如:

```ts
await build({
  // ...etc...
  packageManager: 'yarn', // 或者 "pnpm"
})
```

你甚至可以指定包管理工具执行程序的绝对路径:

```ts
await build({
  // ...etc...
  packageManager: '/usr/bin/pnpm',
})
```

### Node v14 以及更低的版本

dnt 可以通过在构建设置中配置 `{ compilerOption: { target: ... }}` 指定低版本的 Node(详细请参考 [Node Target Mapping](https://github.com/microsoft/TypeScript/wiki/Node-Target-Mapping)来获得参数与 node 版本的映射关系)。但是，要注意很多 shim 在低版本的 Node 是不起作用的。

如果你想指定 Node v14 或者更低的版本，推荐使用 `Deno.test`-only shim，然后通过 “映射” 功能来写仅供 Node 的文件。也可以通过去改变 shim 库来看是否可以跑在低版本的 Node 上。不幸的是，很多特定的功能是不可能或者不完全能跑在低版本的 Node 上的。

请参考 [this thread](https://github.com/denoland/node_deno_shims/issues/15) 来详细了解 node_deno_shims。

## JS API 样例

对于仅仅 Deno 到规范 TypeScript 的代码转换可能对于打包器是有帮助的，可以参考下面的接口:

```ts
// docs: https://doc.deno.land/https/deno.land/x/dnt/transform.ts
import { transform } from 'https://deno.land/x/dnt/transform.ts'

const outputResult = await transform({
  entryPoints: ['./mod.ts'],
  testEntryPoints: ['./mod.test.ts'],
  shims: [],
  testShims: [],
  // mappings: {}, // optional specifier mappings
})
```

## Rust API 样例

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
