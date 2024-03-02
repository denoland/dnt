// @generated file from wasmbuild -- do not edit
// @ts-nocheck: generated
// deno-lint-ignore-file
// deno-fmt-ignore-file
// source-hash: 06863161a3ee5ee9c3cd49086ae94f4108649bbb
let wasm;

import { fetch_specifier } from "./snippets/dnt-wasm-a15ef721fa5290c5/helpers.js";

const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) {
  return heap[idx];
}

let WASM_VECTOR_LEN = 0;

let cachedUint8Memory0 = null;

function getUint8Memory0() {
  if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
    cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
  }
  return cachedUint8Memory0;
}

const cachedTextEncoder = typeof TextEncoder !== "undefined"
  ? new TextEncoder("utf-8")
  : {
    encode: () => {
      throw Error("TextEncoder not available");
    },
  };

const encodeString = function (arg, view) {
  return cachedTextEncoder.encodeInto(arg, view);
};

function passStringToWasm0(arg, malloc, realloc) {
  if (realloc === undefined) {
    const buf = cachedTextEncoder.encode(arg);
    const ptr = malloc(buf.length, 1) >>> 0;
    getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
    WASM_VECTOR_LEN = buf.length;
    return ptr;
  }

  let len = arg.length;
  let ptr = malloc(len, 1) >>> 0;

  const mem = getUint8Memory0();

  let offset = 0;

  for (; offset < len; offset++) {
    const code = arg.charCodeAt(offset);
    if (code > 0x7F) break;
    mem[ptr + offset] = code;
  }

  if (offset !== len) {
    if (offset !== 0) {
      arg = arg.slice(offset);
    }
    ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
    const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
    const ret = encodeString(arg, view);

    offset += ret.written;
    ptr = realloc(ptr, len, offset, 1) >>> 0;
  }

  WASM_VECTOR_LEN = offset;
  return ptr;
}

let cachedInt32Memory0 = null;

function getInt32Memory0() {
  if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
    cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
  }
  return cachedInt32Memory0;
}

function isLikeNone(x) {
  return x === undefined || x === null;
}

const cachedTextDecoder = typeof TextDecoder !== "undefined"
  ? new TextDecoder("utf-8", { ignoreBOM: true, fatal: true })
  : {
    decode: () => {
      throw Error("TextDecoder not available");
    },
  };

if (typeof TextDecoder !== "undefined") cachedTextDecoder.decode();

function getStringFromWasm0(ptr, len) {
  ptr = ptr >>> 0;
  return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let heap_next = heap.length;

function addHeapObject(obj) {
  if (heap_next === heap.length) heap.push(heap.length + 1);
  const idx = heap_next;
  heap_next = heap[idx];

  heap[idx] = obj;
  return idx;
}

function dropObject(idx) {
  if (idx < 132) return;
  heap[idx] = heap_next;
  heap_next = idx;
}

function takeObject(idx) {
  const ret = getObject(idx);
  dropObject(idx);
  return ret;
}

let cachedFloat64Memory0 = null;

function getFloat64Memory0() {
  if (cachedFloat64Memory0 === null || cachedFloat64Memory0.byteLength === 0) {
    cachedFloat64Memory0 = new Float64Array(wasm.memory.buffer);
  }
  return cachedFloat64Memory0;
}

function debugString(val) {
  // primitive types
  const type = typeof val;
  if (type == "number" || type == "boolean" || val == null) {
    return `${val}`;
  }
  if (type == "string") {
    return `"${val}"`;
  }
  if (type == "symbol") {
    const description = val.description;
    if (description == null) {
      return "Symbol";
    } else {
      return `Symbol(${description})`;
    }
  }
  if (type == "function") {
    const name = val.name;
    if (typeof name == "string" && name.length > 0) {
      return `Function(${name})`;
    } else {
      return "Function";
    }
  }
  // objects
  if (Array.isArray(val)) {
    const length = val.length;
    let debug = "[";
    if (length > 0) {
      debug += debugString(val[0]);
    }
    for (let i = 1; i < length; i++) {
      debug += ", " + debugString(val[i]);
    }
    debug += "]";
    return debug;
  }
  // Test for built-in
  const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
  let className;
  if (builtInMatches.length > 1) {
    className = builtInMatches[1];
  } else {
    // Failed to match the standard '[object ClassName]'
    return toString.call(val);
  }
  if (className == "Object") {
    // we're a user defined class or Object
    // JSON.stringify avoids problems with cycles, and is generally much
    // easier than looping through ownProperties of `val`.
    try {
      return "Object(" + JSON.stringify(val) + ")";
    } catch (_) {
      return "Object";
    }
  }
  // errors
  if (val instanceof Error) {
    return `${val.name}: ${val.message}\n${val.stack}`;
  }
  // TODO we could test for more things here, like `Set`s and `Map`s.
  return className;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === "undefined")
  ? { register: () => {}, unregister: () => {} }
  : new FinalizationRegistry((state) => {
    wasm.__wbindgen_export_2.get(state.dtor)(state.a, state.b);
  });

function makeMutClosure(arg0, arg1, dtor, f) {
  const state = { a: arg0, b: arg1, cnt: 1, dtor };
  const real = (...args) => {
    // First up with a closure we increment the internal reference
    // count. This ensures that the Rust closure environment won't
    // be deallocated while we're invoking it.
    state.cnt++;
    const a = state.a;
    state.a = 0;
    try {
      return f(a, state.b, ...args);
    } finally {
      if (--state.cnt === 0) {
        wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);
        CLOSURE_DTORS.unregister(state);
      } else {
        state.a = a;
      }
    }
  };
  real.original = state;
  CLOSURE_DTORS.register(real, state, state);
  return real;
}
function __wbg_adapter_38(arg0, arg1, arg2) {
  wasm.wasm_bindgen__convert__closures__invoke1_mut__h5ed013030774d99a(
    arg0,
    arg1,
    addHeapObject(arg2),
  );
}

/**
 * @param {any} options
 * @returns {Promise<any>}
 */
export function transform(options) {
  const ret = wasm.transform(addHeapObject(options));
  return takeObject(ret);
}

function handleError(f, args) {
  try {
    return f.apply(this, args);
  } catch (e) {
    wasm.__wbindgen_exn_store(addHeapObject(e));
  }
}
function __wbg_adapter_84(arg0, arg1, arg2, arg3) {
  wasm.wasm_bindgen__convert__closures__invoke2_mut__h537f8faf29c265b4(
    arg0,
    arg1,
    addHeapObject(arg2),
    addHeapObject(arg3),
  );
}

const imports = {
  __wbindgen_placeholder__: {
    __wbindgen_json_serialize: function (arg0, arg1) {
      const obj = getObject(arg1);
      const ret = JSON.stringify(obj === undefined ? null : obj);
      const ptr1 = passStringToWasm0(
        ret,
        wasm.__wbindgen_malloc,
        wasm.__wbindgen_realloc,
      );
      const len1 = WASM_VECTOR_LEN;
      getInt32Memory0()[arg0 / 4 + 1] = len1;
      getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    },
    __wbindgen_is_undefined: function (arg0) {
      const ret = getObject(arg0) === undefined;
      return ret;
    },
    __wbindgen_string_get: function (arg0, arg1) {
      const obj = getObject(arg1);
      const ret = typeof obj === "string" ? obj : undefined;
      var ptr1 = isLikeNone(ret)
        ? 0
        : passStringToWasm0(
          ret,
          wasm.__wbindgen_malloc,
          wasm.__wbindgen_realloc,
        );
      var len1 = WASM_VECTOR_LEN;
      getInt32Memory0()[arg0 / 4 + 1] = len1;
      getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    },
    __wbindgen_is_object: function (arg0) {
      const val = getObject(arg0);
      const ret = typeof val === "object" && val !== null;
      return ret;
    },
    __wbindgen_error_new: function (arg0, arg1) {
      const ret = new Error(getStringFromWasm0(arg0, arg1));
      return addHeapObject(ret);
    },
    __wbindgen_object_drop_ref: function (arg0) {
      takeObject(arg0);
    },
    __wbindgen_in: function (arg0, arg1) {
      const ret = getObject(arg0) in getObject(arg1);
      return ret;
    },
    __wbindgen_string_new: function (arg0, arg1) {
      const ret = getStringFromWasm0(arg0, arg1);
      return addHeapObject(ret);
    },
    __wbg_fetchspecifier_a6bda29173284144: function (arg0, arg1, arg2) {
      let deferred0_0;
      let deferred0_1;
      try {
        deferred0_0 = arg0;
        deferred0_1 = arg1;
        const ret = fetch_specifier(getStringFromWasm0(arg0, arg1), arg2);
        return addHeapObject(ret);
      } finally {
        wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
      }
    },
    __wbindgen_is_null: function (arg0) {
      const ret = getObject(arg0) === null;
      return ret;
    },
    __wbindgen_number_get: function (arg0, arg1) {
      const obj = getObject(arg1);
      const ret = typeof obj === "number" ? obj : undefined;
      getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
      getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    },
    __wbindgen_object_clone_ref: function (arg0) {
      const ret = getObject(arg0);
      return addHeapObject(ret);
    },
    __wbindgen_jsval_loose_eq: function (arg0, arg1) {
      const ret = getObject(arg0) == getObject(arg1);
      return ret;
    },
    __wbindgen_boolean_get: function (arg0) {
      const v = getObject(arg0);
      const ret = typeof v === "boolean" ? (v ? 1 : 0) : 2;
      return ret;
    },
    __wbg_getwithrefkey_5e6d9547403deab8: function (arg0, arg1) {
      const ret = getObject(arg0)[getObject(arg1)];
      return addHeapObject(ret);
    },
    __wbg_set_841ac57cff3d672b: function (arg0, arg1, arg2) {
      getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
    },
    __wbg_queueMicrotask_f82fc5d1e8f816ae: function (arg0) {
      const ret = getObject(arg0).queueMicrotask;
      return addHeapObject(ret);
    },
    __wbindgen_is_function: function (arg0) {
      const ret = typeof (getObject(arg0)) === "function";
      return ret;
    },
    __wbindgen_cb_drop: function (arg0) {
      const obj = takeObject(arg0).original;
      if (obj.cnt-- == 1) {
        obj.a = 0;
        return true;
      }
      const ret = false;
      return ret;
    },
    __wbg_queueMicrotask_f61ee94ee663068b: function (arg0) {
      queueMicrotask(getObject(arg0));
    },
    __wbg_self_05040bd9523805b9: function () {
      return handleError(function () {
        const ret = self.self;
        return addHeapObject(ret);
      }, arguments);
    },
    __wbg_window_adc720039f2cb14f: function () {
      return handleError(function () {
        const ret = window.window;
        return addHeapObject(ret);
      }, arguments);
    },
    __wbg_globalThis_622105db80c1457d: function () {
      return handleError(function () {
        const ret = globalThis.globalThis;
        return addHeapObject(ret);
      }, arguments);
    },
    __wbg_global_f56b013ed9bcf359: function () {
      return handleError(function () {
        const ret = global.global;
        return addHeapObject(ret);
      }, arguments);
    },
    __wbg_newnoargs_cfecb3965268594c: function (arg0, arg1) {
      const ret = new Function(getStringFromWasm0(arg0, arg1));
      return addHeapObject(ret);
    },
    __wbg_length_161c0d89c6535c1d: function (arg0) {
      const ret = getObject(arg0).length;
      return ret;
    },
    __wbg_new_75208e29bddfd88c: function () {
      const ret = new Array();
      return addHeapObject(ret);
    },
    __wbg_next_586204376d2ed373: function (arg0) {
      const ret = getObject(arg0).next;
      return addHeapObject(ret);
    },
    __wbg_value_3158be908c80a75e: function (arg0) {
      const ret = getObject(arg0).value;
      return addHeapObject(ret);
    },
    __wbg_iterator_40027cdd598da26b: function () {
      const ret = Symbol.iterator;
      return addHeapObject(ret);
    },
    __wbg_new_632630b5cec17f21: function () {
      const ret = new Object();
      return addHeapObject(ret);
    },
    __wbg_get_0ee8ea3c7c984c45: function (arg0, arg1) {
      const ret = getObject(arg0)[arg1 >>> 0];
      return addHeapObject(ret);
    },
    __wbg_set_79c308ecd9a1d091: function (arg0, arg1, arg2) {
      getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
    },
    __wbg_instanceof_ArrayBuffer_9221fa854ffb71b5: function (arg0) {
      let result;
      try {
        result = getObject(arg0) instanceof ArrayBuffer;
      } catch (_) {
        result = false;
      }
      const ret = result;
      return ret;
    },
    __wbg_call_3f093dd26d5569f8: function () {
      return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
      }, arguments);
    },
    __wbg_call_67f2111acd2dfdb6: function () {
      return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
      }, arguments);
    },
    __wbg_next_b2d3366343a208b3: function () {
      return handleError(function (arg0) {
        const ret = getObject(arg0).next();
        return addHeapObject(ret);
      }, arguments);
    },
    __wbg_done_90b14d6f6eacc42f: function (arg0) {
      const ret = getObject(arg0).done;
      return ret;
    },
    __wbg_entries_488960b196cfb6a5: function (arg0) {
      const ret = Object.entries(getObject(arg0));
      return addHeapObject(ret);
    },
    __wbg_get_3fddfed2c83f434c: function () {
      return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
      }, arguments);
    },
    __wbg_buffer_b914fb8b50ebbc3e: function (arg0) {
      const ret = getObject(arg0).buffer;
      return addHeapObject(ret);
    },
    __wbg_new_70828a4353259d4b: function (arg0, arg1) {
      try {
        var state0 = { a: arg0, b: arg1 };
        var cb0 = (arg0, arg1) => {
          const a = state0.a;
          state0.a = 0;
          try {
            return __wbg_adapter_84(a, state0.b, arg0, arg1);
          } finally {
            state0.a = a;
          }
        };
        const ret = new Promise(cb0);
        return addHeapObject(ret);
      } finally {
        state0.a = state0.b = 0;
      }
    },
    __wbg_resolve_5da6faf2c96fd1d5: function (arg0) {
      const ret = Promise.resolve(getObject(arg0));
      return addHeapObject(ret);
    },
    __wbg_then_f9e58f5a50f43eae: function (arg0, arg1) {
      const ret = getObject(arg0).then(getObject(arg1));
      return addHeapObject(ret);
    },
    __wbg_then_20a5920e447d1cb1: function (arg0, arg1, arg2) {
      const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
      return addHeapObject(ret);
    },
    __wbg_new_b1f2d6842d615181: function (arg0) {
      const ret = new Uint8Array(getObject(arg0));
      return addHeapObject(ret);
    },
    __wbg_instanceof_Uint8Array_c299a4ee232e76ba: function (arg0) {
      let result;
      try {
        result = getObject(arg0) instanceof Uint8Array;
      } catch (_) {
        result = false;
      }
      const ret = result;
      return ret;
    },
    __wbg_length_21c4b0ae73cba59d: function (arg0) {
      const ret = getObject(arg0).length;
      return ret;
    },
    __wbg_set_7d988c98e6ced92d: function (arg0, arg1, arg2) {
      getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    },
    __wbg_new_abda76e883ba8a5f: function () {
      const ret = new Error();
      return addHeapObject(ret);
    },
    __wbg_stack_658279fe44541cf6: function (arg0, arg1) {
      const ret = getObject(arg1).stack;
      const ptr1 = passStringToWasm0(
        ret,
        wasm.__wbindgen_malloc,
        wasm.__wbindgen_realloc,
      );
      const len1 = WASM_VECTOR_LEN;
      getInt32Memory0()[arg0 / 4 + 1] = len1;
      getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    },
    __wbg_error_f851667af71bcfc6: function (arg0, arg1) {
      let deferred0_0;
      let deferred0_1;
      try {
        deferred0_0 = arg0;
        deferred0_1 = arg1;
        console.error(getStringFromWasm0(arg0, arg1));
      } finally {
        wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
      }
    },
    __wbindgen_debug_string: function (arg0, arg1) {
      const ret = debugString(getObject(arg1));
      const ptr1 = passStringToWasm0(
        ret,
        wasm.__wbindgen_malloc,
        wasm.__wbindgen_realloc,
      );
      const len1 = WASM_VECTOR_LEN;
      getInt32Memory0()[arg0 / 4 + 1] = len1;
      getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    },
    __wbindgen_throw: function (arg0, arg1) {
      throw new Error(getStringFromWasm0(arg0, arg1));
    },
    __wbindgen_memory: function () {
      const ret = wasm.memory;
      return addHeapObject(ret);
    },
    __wbindgen_closure_wrapper910: function (arg0, arg1, arg2) {
      const ret = makeMutClosure(arg0, arg1, 131, __wbg_adapter_38);
      return addHeapObject(ret);
    },
  },
};

/**
 * @callback WasmBuildDecompressCallback
 * @param {Uint8Array} compressed
 * @returns {Uint8Array} decompressed
 */

/**
 * @callback WasmBuildCacheCallback
 * @param {URL} url
 * @param {WasmBuildDecompressCallback | undefined} decompress
 * @returns {Promise<URL |Uint8Array>}
 */

/**
 * @typedef WasmBuildLoaderOptions
 * @property {WebAssembly.Imports | undefined} imports - The Wasm module's imports.
 * @property {WasmBuildCacheCallback} [cache] - A function that caches the Wasm module to
 * a local path so that a network request isn't required on every load.
 *
 * Returns an ArrayBuffer with the bytes on download success, but cache save failure.
 */

class WasmBuildLoader {
  /** @type {WasmBuildLoaderOptions} */
  #options;
  /** @type {Promise<WebAssembly.WebAssemblyInstantiatedSource> | undefined} */
  #lastLoadPromise;
  /** @type {WebAssembly.WebAssemblyInstantiatedSource | undefined} */
  #instantiated;

  /** @param {WasmBuildLoaderOptions} options */
  constructor(options) {
    this.#options = options;
  }

  /** @returns {WebAssembly.Instance | undefined} */
  get instance() {
    return this.#instantiated?.instance;
  }

  /** @returns {WebAssembly.Module | undefined} */
  get module() {
    return this.#instantiated?.module;
  }

  /**
   * @param {URL} url
   * @param {WasmBuildDecompressCallback | undefined} decompress
   * @returns {Promise<WebAssembly.WebAssemblyInstantiatedSource>}
   */
  load(
    url,
    decompress,
  ) {
    if (this.#instantiated) {
      return Promise.resolve(this.#instantiated);
    } else if (this.#lastLoadPromise == null) {
      this.#lastLoadPromise = (async () => {
        try {
          this.#instantiated = await this.#instantiate(url, decompress);
          return this.#instantiated;
        } finally {
          this.#lastLoadPromise = undefined;
        }
      })();
    }
    return this.#lastLoadPromise;
  }

  /**
   * @param {URL} url
   * @param {WasmBuildDecompressCallback | undefined} decompress
   */
  async #instantiate(url, decompress) {
    const imports = this.#options.imports;
    if (this.#options.cache != null && url.protocol !== "file:") {
      try {
        const result = await this.#options.cache(
          url,
          decompress ?? ((bytes) => bytes),
        );
        if (result instanceof URL) {
          url = result;
          decompress = undefined; // already decompressed
        } else if (result != null) {
          return WebAssembly.instantiate(result, imports);
        }
      } catch {
        // ignore if caching ever fails (ex. when on deploy)
      }
    }

    const isFile = url.protocol === "file:";

    // make file urls work in Node via dnt
    const isNode =
      (/** @type {any} */ (globalThis)).process?.versions?.node != null;
    if (isFile && typeof Deno !== "object") {
      throw new Error(
        "Loading local files are not supported in this environment",
      );
    }
    if (isNode && isFile) {
      // the deno global will be shimmed by dnt
      const wasmCode = await Deno.readFile(url);
      return WebAssembly.instantiate(
        decompress ? decompress(wasmCode) : wasmCode,
        imports,
      );
    }

    switch (url.protocol) {
      case "file:":
      case "https:":
      case "http:": {
        const wasmResponse = await fetchWithRetries(url);
        if (decompress) {
          const wasmCode = new Uint8Array(await wasmResponse.arrayBuffer());
          return WebAssembly.instantiate(decompress(wasmCode), imports);
        }
        if (
          isFile ||
          wasmResponse.headers.get("content-type")?.toLowerCase()
            .startsWith("application/wasm")
        ) {
          return WebAssembly.instantiateStreaming(
            // Cast to any so there's no type checking issues with dnt
            // (https://github.com/denoland/wasmbuild/issues/92)
            /** @type {any} */ (wasmResponse),
            imports,
          );
        } else {
          return WebAssembly.instantiate(
            await wasmResponse.arrayBuffer(),
            imports,
          );
        }
      }
      default:
        throw new Error(`Unsupported protocol: ${url.protocol}`);
    }
  }
}
const isNodeOrDeno = typeof Deno === "object" ||
  (typeof process !== "undefined" && process.versions != null &&
    process.versions.node != null);

const loader = new WasmBuildLoader({
  imports,
  cache: isNodeOrDeno ? cacheToLocalDir : undefined,
});
/**
 * Options for instantiating a Wasm instance.
 * @typedef {Object} InstantiateOptions
 * @property {URL=} url - Optional url to the Wasm file to instantiate.
 * @property {DecompressCallback=} decompress - Callback to decompress the
 * raw Wasm file bytes before instantiating.
 */

/** Instantiates an instance of the Wasm module returning its functions.
 * @remarks It is safe to call this multiple times and once successfully
 * loaded it will always return a reference to the same object.
 * @param {InstantiateOptions=} opts
 */
export async function instantiate(opts) {
  return (await instantiateWithInstance(opts)).exports;
}

/** Instantiates an instance of the Wasm module along with its exports.
 * @remarks It is safe to call this multiple times and once successfully
 * loaded it will always return a reference to the same object.
 * @param {InstantiateOptions=} opts
 * @returns {Promise<{
 *   instance: WebAssembly.Instance;
 *   exports: { transform: typeof transform }
 * }>}
 */
export async function instantiateWithInstance(opts) {
  const { instance } = await loader.load(
    opts?.url ?? new URL("dnt_wasm_bg.wasm", import.meta.url),
    opts?.decompress,
  );
  wasm = wasm ?? instance.exports;
  cachedInt32Memory0 = cachedInt32Memory0 ?? new Int32Array(wasm.memory.buffer);
  cachedUint8Memory0 = cachedUint8Memory0 ?? new Uint8Array(wasm.memory.buffer);
  return {
    instance,
    exports: getWasmInstanceExports(),
  };
}

function getWasmInstanceExports() {
  return { transform };
}

/** Gets if the Wasm module has been instantiated. */
export function isInstantiated() {
  return loader.instance != null;
}
export async function cacheToLocalDir(url, decompress) {
  const localPath = await getUrlLocalPath(url);
  if (localPath == null) {
    return undefined;
  }
  if (!await exists(localPath)) {
    const fileBytes = decompress(new Uint8Array(await getUrlBytes(url)));
    try {
      await Deno.writeFile(localPath, fileBytes);
    } catch {
      // ignore and return the wasm bytes
      return fileBytes;
    }
  }
  return toFileUrl(localPath);
}
async function getUrlLocalPath(url) {
  try {
    const dataDirPath = await getInitializedLocalDataDirPath();
    const hash = await getUrlHash(url);
    return `${dataDirPath}/${hash}.wasm`;
  } catch {
    return undefined;
  }
}
async function getInitializedLocalDataDirPath() {
  const dataDir = localDataDir();
  if (dataDir == null) {
    throw new Error(`Could not find local data directory.`);
  }
  const dirPath = `${dataDir}/deno-wasmbuild`;
  await ensureDir(dirPath);
  return dirPath;
}
async function exists(filePath) {
  try {
    await Deno.lstat(filePath);
    return true;
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) {
      return false;
    }
    throw error;
  }
}
async function ensureDir(dir) {
  try {
    const fileInfo = await Deno.lstat(dir);
    if (!fileInfo.isDirectory) {
      throw new Error(`Path was not a directory '${dir}'`);
    }
  } catch (err) {
    if (err instanceof Deno.errors.NotFound) {
      // if dir not exists. then create it.
      await Deno.mkdir(dir, { recursive: true });
      return;
    }
    throw err;
  }
}
async function getUrlHash(url) {
  // Taken from MDN: https://developer.mozilla.org/en-US/docs/Web/API/SubtleCrypto/digest
  const hashBuffer = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(url.href),
  );
  // convert buffer to byte array
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  // convert bytes to hex string
  const hashHex = hashArray
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return hashHex;
}
async function getUrlBytes(url) {
  const response = await fetchWithRetries(url);
  return await response.arrayBuffer();
}
// the below is extracted from deno_std/path
const WHITESPACE_ENCODINGS = {
  "\u0009": "%09",
  "\u000A": "%0A",
  "\u000B": "%0B",
  "\u000C": "%0C",
  "\u000D": "%0D",
  "\u0020": "%20",
};
function encodeWhitespace(string) {
  return string.replaceAll(/[\s]/g, (c) => {
    return WHITESPACE_ENCODINGS[c] ?? c;
  });
}
function toFileUrl(path) {
  return Deno.build.os === "windows"
    ? windowsToFileUrl(path)
    : posixToFileUrl(path);
}
function posixToFileUrl(path) {
  const url = new URL("file:///");
  url.pathname = encodeWhitespace(
    path.replace(/%/g, "%25").replace(/\\/g, "%5C"),
  );
  return url;
}
function windowsToFileUrl(path) {
  const [, hostname, pathname] = path.match(
    /^(?:[/\\]{2}([^/\\]+)(?=[/\\](?:[^/\\]|$)))?(.*)/,
  );
  const url = new URL("file:///");
  url.pathname = encodeWhitespace(pathname.replace(/%/g, "%25"));
  if (hostname != null && hostname != "localhost") {
    url.hostname = hostname;
    if (!url.hostname) {
      throw new TypeError("Invalid hostname.");
    }
  }
  return url;
}
export async function fetchWithRetries(url, maxRetries = 5) {
  let sleepMs = 250;
  let iterationCount = 0;
  while (true) {
    iterationCount++;
    try {
      const res = await fetch(url);
      if (res.ok || iterationCount > maxRetries) {
        return res;
      }
    } catch (err) {
      if (iterationCount > maxRetries) {
        throw err;
      }
    }
    console.warn(`Failed fetching. Retrying in ${sleepMs}ms...`);
    await new Promise((resolve) => setTimeout(resolve, sleepMs));
    sleepMs = Math.min(sleepMs * 2, 10000);
  }
}
// MIT License - Copyright (c) justjavac.
// https://github.com/justjavac/deno_dirs/blob/e8c001bbef558f08fd486d444af391729b0b8068/data_local_dir/mod.ts
function localDataDir() {
  switch (Deno.build.os) {
    case "linux": {
      const xdg = Deno.env.get("XDG_DATA_HOME");
      if (xdg) {
        return xdg;
      }
      const home = Deno.env.get("HOME");
      if (home) {
        return `${home}/.local/share`;
      }
      break;
    }
    case "darwin": {
      const home = Deno.env.get("HOME");
      if (home) {
        return `${home}/Library/Application Support`;
      }
      break;
    }
    case "windows":
      return Deno.env.get("LOCALAPPDATA") ?? undefined;
  }
  return undefined;
}
