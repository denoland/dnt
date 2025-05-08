/**
 * Based on [import-meta-ponyfill](https://github.com/gaubee/import-meta-ponyfill),
 * but instead of using npm to install additional dependencies,
 * this approach manually consolidates cjs/mjs/d.ts into a single file.
 *
 * Note that this code might be imported multiple times
 * (for example, both dnt.test.polyfills.ts and dnt.polyfills.ts contain this code;
 *  or Node.js might dynamically clear the cache and then force a require).
 * Therefore, it's important to avoid redundant writes to global objects.
 * Additionally, consider that commonjs is used alongside esm,
 * so the two ponyfill functions are stored independently in two separate global objects.
 */
//@ts-ignore
import { createRequire } from "node:module";
//@ts-ignore
import { fileURLToPath, pathToFileURL, type URL } from "node:url";
//@ts-ignore
import { dirname } from "node:path";
declare global {
  interface ImportMeta {
    /** A string representation of the fully qualified module URL. When the
     * module is loaded locally, the value will be a file URL (e.g.
     * `file:///path/module.ts`).
     *
     * You can also parse the string as a URL to determine more information about
     * how the current module was loaded. For example to determine if a module was
     * local or not:
     *
     * ```ts
     * const url = new URL(import.meta.url);
     * if (url.protocol === "file:") {
     *   console.log("this module was loaded locally");
     * }
     * ```
     */
    url: string;
    /**
     * A function that returns resolved specifier as if it would be imported
     * using `import(specifier)`.
     *
     * ```ts
     * console.log(import.meta.resolve("./foo.js"));
     * // file:///dev/foo.js
     * ```
     *
     * @param specifier The module specifier to resolve relative to `parent`.
     * @param parent The absolute parent module URL to resolve from.
     * @returns The absolute (`file:`) URL string for the resolved module.
     */
    resolve(specifier: string, parent?: string | URL | undefined): string;
    /** A flag that indicates if the current module is the main module that was
     * called when starting the program under Deno.
     *
     * ```ts
     * if (import.meta.main) {
     *   // this was loaded as the main module, maybe do some bootstrapping
     * }
     * ```
     */
    main: boolean;

    /** The absolute path of the current module.
     *
     * This property is only provided for local modules (ie. using `file://` URLs).
     *
     * Example:
     * ```
     * // Unix
     * console.log(import.meta.filename); // /home/alice/my_module.ts
     *
     * // Windows
     * console.log(import.meta.filename); // C:\alice\my_module.ts
     * ```
     */
    filename: string;

    /** The absolute path of the directory containing the current module.
     *
     * This property is only provided for local modules (ie. using `file://` URLs).
     *
     * * Example:
     * ```
     * // Unix
     * console.log(import.meta.dirname); // /home/alice
     *
     * // Windows
     * console.log(import.meta.dirname); // C:\alice
     * ```
     */
    dirname: string;
  }
}

type NodeRequest = ReturnType<typeof createRequire>;
type NodeModule = NonNullable<NodeRequest["main"]>;
interface ImportMetaPonyfillCommonjs {
  (require: NodeRequest, module: NodeModule): ImportMeta;
}
interface ImportMetaPonyfillEsmodule {
  (importMeta: ImportMeta): ImportMeta;
}
interface ImportMetaPonyfill
  extends ImportMetaPonyfillCommonjs, ImportMetaPonyfillEsmodule {
}

const defineGlobalPonyfill = (symbolFor: string, fn: Function) => {
  if (!Reflect.has(globalThis, Symbol.for(symbolFor))) {
    Object.defineProperty(
      globalThis,
      Symbol.for(symbolFor),
      {
        configurable: true,
        get() {
          return fn;
        },
      },
    );
  }
};

export let import_meta_ponyfill_commonjs = (
  Reflect.get(globalThis, Symbol.for("import-meta-ponyfill-commonjs")) ??
    (() => {
      const moduleImportMetaWM = new WeakMap<NodeModule, ImportMeta>();
      return (require, module) => {
        let importMetaCache = moduleImportMetaWM.get(module);
        if (importMetaCache == null) {
          const importMeta = Object.assign(Object.create(null), {
            url: pathToFileURL(module.filename).href,
            main: require.main == module,
            resolve: (specifier: string, parentURL = importMeta.url) => {
              return pathToFileURL(
                (importMeta.url === parentURL
                  ? require
                  : createRequire(parentURL))
                  .resolve(specifier),
              ).href;
            },
            filename: module.filename,
            dirname: module.path,
          });
          moduleImportMetaWM.set(module, importMeta);
          importMetaCache = importMeta;
        }
        return importMetaCache;
      };
    })()
) as ImportMetaPonyfillCommonjs;
defineGlobalPonyfill(
  "import-meta-ponyfill-commonjs",
  import_meta_ponyfill_commonjs,
);

export let import_meta_ponyfill_esmodule = (
  Reflect.get(globalThis, Symbol.for("import-meta-ponyfill-esmodule")) ??
    ((importMeta: ImportMeta) => {
      const resolveFunStr = String(importMeta.resolve);
      const shimWs = new WeakSet();
      //@ts-ignore
      const mainUrl = ("file:///" + process.argv[1].replace(/\\/g, "/"))
        .replace(
          /\/{3,}/,
          "///",
        );
      const commonShim = (importMeta: ImportMeta) => {
        if (typeof importMeta.main !== "boolean") {
          importMeta.main = importMeta.url === mainUrl;
        }
        if (typeof importMeta.filename !== "string") {
          importMeta.filename = fileURLToPath(importMeta.url);
          importMeta.dirname = dirname(importMeta.filename);
        }
      };
      if (
        // v16.2.0+, v14.18.0+: Add support for WHATWG URL object to parentURL parameter.
        resolveFunStr === "undefined" ||
        // v20.0.0+, v18.19.0+"" This API now returns a string synchronously instead of a Promise.
        resolveFunStr.startsWith("async")
        // enable by --experimental-import-meta-resolve flag
      ) {
        import_meta_ponyfill_esmodule = (importMeta: ImportMeta) => {
          if (!shimWs.has(importMeta)) {
            shimWs.add(importMeta);
            const importMetaUrlRequire = {
              url: importMeta.url,
              require: createRequire(importMeta.url),
            };
            importMeta.resolve = function resolve(
              specifier: string,
              parentURL = importMeta.url,
            ) {
              return pathToFileURL(
                (importMetaUrlRequire.url === parentURL
                  ? importMetaUrlRequire.require
                  : createRequire(parentURL)).resolve(specifier),
              ).href;
            };
            commonShim(importMeta);
          }
          return importMeta;
        };
      } else {
        /// native support
        import_meta_ponyfill_esmodule = (importMeta: ImportMeta) => {
          if (!shimWs.has(importMeta)) {
            shimWs.add(importMeta);
            commonShim(importMeta);
          }
          return importMeta;
        };
      }
      return import_meta_ponyfill_esmodule(importMeta);
    })
) as ImportMetaPonyfillEsmodule;
defineGlobalPonyfill(
  "import-meta-ponyfill-esmodule",
  import_meta_ponyfill_esmodule,
);

export let import_meta_ponyfill = (
  (...args: any[]) => {
    const _MODULE = (() => {
      if (typeof require === "function" && typeof module === "object") {
        return "commonjs";
      } else {
        // eval("typeof import.meta");
        return "esmodule";
      }
    })();
    if (_MODULE === "commonjs") {
      //@ts-ignore
      import_meta_ponyfill = (r, m) => import_meta_ponyfill_commonjs(r, m);
    } else {
      //@ts-ignore
      import_meta_ponyfill = (im) => import_meta_ponyfill_esmodule(im);
    }
    //@ts-ignore
    return import_meta_ponyfill(...args);
  }
) as ImportMetaPonyfill;
