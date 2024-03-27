// Copyright 2018-2024 the Deno authors. MIT license.

import type { GlobalName, Shim } from "../transform.ts";

/** Provide `true` to use the shim in both the distributed code and test code,
 * `"dev"` to only use it in the test code, or `false` to not use the shim
 * at all.
 *
 * @remarks Defaults to `false`.
 */
export type ShimValue = boolean | "dev";

/** Provide `true` to use the shim in both the distributed code and test code,
 * `"dev"` to only use it in the test code, or `false` to not use the shim
 * at all.
 *
 * @remarks These all default to `false`.
 */
export interface ShimOptions {
  /** Shim the `Deno` namespace. */
  deno?: ShimValue | {
    test: ShimValue;
  };
  /** Shim the global `setTimeout` and `setInterval` functions with
   * Deno and browser compatible versions.
   */
  timers?: ShimValue;
  /** Shim the global `confirm`, `alert`, and `prompt` functions. */
  prompts?: ShimValue;
  /** Shim the `Blob` global with the one from the `"buffer"` module. */
  blob?: ShimValue;
  /** Shim the `crypto` global. */
  crypto?: ShimValue;
  /** Shim `DOMException` using the "domexception" package (https://www.npmjs.com/package/domexception) */
  domException?: ShimValue;
  /** Shim `fetch`, `File`, `FormData`, `Headers`, `Request`, and `Response` by
   * using the "undici" package (https://www.npmjs.com/package/undici).
   */
  undici?: ShimValue;
  /** Use a sham for the `WeakRef` global, which uses `globalThis.WeakRef` when
   * it exists. The sham will throw at runtime when calling `deref()` and `WeakRef`
   * doesn't globally exist, so this is only intended to help type check code that
   * won't actually use it.
   */
  weakRef?: ShimValue;
  /** Shim `WebSocket` with the `ws` package (https://www.npmjs.com/package/ws). */
  webSocket?: boolean | "dev";
  /** Custom shims to use. */
  custom?: Shim[];
  /** Custom shims to use only for the test code. */
  customDev?: Shim[];
}

export interface DenoShimOptions {
  /** Only import the `Deno` namespace for `Deno.test`.
   * This may be useful for environments
   */
  test: boolean | "dev";
}

export function shimOptionsToTransformShims(options: ShimOptions) {
  const shims: Shim[] = [];
  const testShims: Shim[] = [];

  if (typeof options.deno === "object") {
    add(options.deno.test, getDenoTestShim);
  } else {
    add(options.deno, getDenoShim);
  }
  add(options.blob, getBlobShim);
  add(options.crypto, getCryptoShim);
  add(options.prompts, getPromptsShim);
  add(options.timers, getTimersShim);
  add(options.domException, getDomExceptionShim);
  add(options.undici, getUndiciShim);
  add(options.weakRef, getWeakRefShim);
  add(options.webSocket, getWebSocketShim);

  if (options.custom) {
    shims.push(...options.custom);
    testShims.push(...options.custom);
  }
  if (options.customDev) {
    testShims.push(...options.customDev);
  }

  return {
    shims,
    testShims,
  };

  function add(option: boolean | "dev" | undefined, getShim: () => Shim) {
    if (option === true) {
      shims.push(getShim());
      testShims.push(getShim());
    } else if (option === "dev") {
      testShims.push(getShim());
    }
  }
}

function getDenoShim(): Shim {
  return {
    package: {
      name: "@deno/shim-deno",
      version: "~0.18.0",
    },
    globalNames: ["Deno"],
  };
}

function getDenoTestShim(): Shim {
  return {
    package: {
      name: "@deno/shim-deno-test",
      version: "~0.5.0",
    },
    globalNames: ["Deno"],
  };
}

function getCryptoShim(): Shim {
  return {
    package: {
      name: "@deno/shim-crypto",
      version: "~0.3.1",
    },
    globalNames: [
      "crypto",
      typeOnly("Crypto"),
      typeOnly("SubtleCrypto"),
      typeOnly("AlgorithmIdentifier"),
      typeOnly("Algorithm"),
      typeOnly("RsaOaepParams"),
      typeOnly("BufferSource"),
      typeOnly("AesCtrParams"),
      typeOnly("AesCbcParams"),
      typeOnly("AesGcmParams"),
      typeOnly("CryptoKey"),
      typeOnly("KeyAlgorithm"),
      typeOnly("KeyType"),
      typeOnly("KeyUsage"),
      typeOnly("EcdhKeyDeriveParams"),
      typeOnly("HkdfParams"),
      typeOnly("HashAlgorithmIdentifier"),
      typeOnly("Pbkdf2Params"),
      typeOnly("AesDerivedKeyParams"),
      typeOnly("HmacImportParams"),
      typeOnly("JsonWebKey"),
      typeOnly("RsaOtherPrimesInfo"),
      typeOnly("KeyFormat"),
      typeOnly("RsaHashedKeyGenParams"),
      typeOnly("RsaKeyGenParams"),
      typeOnly("BigInteger"),
      typeOnly("EcKeyGenParams"),
      typeOnly("NamedCurve"),
      typeOnly("CryptoKeyPair"),
      typeOnly("AesKeyGenParams"),
      typeOnly("HmacKeyGenParams"),
      typeOnly("RsaHashedImportParams"),
      typeOnly("EcKeyImportParams"),
      typeOnly("AesKeyAlgorithm"),
      typeOnly("RsaPssParams"),
      typeOnly("EcdsaParams"),
    ],
  };
}

function getBlobShim(): Shim {
  return {
    module: "buffer",
    globalNames: ["Blob"],
  };
}

function getPromptsShim(): Shim {
  return {
    package: {
      name: "@deno/shim-prompts",
      version: "~0.1.0",
    },
    globalNames: ["alert", "confirm", "prompt"],
  };
}

function getTimersShim(): Shim {
  return {
    package: {
      name: "@deno/shim-timers",
      version: "~0.1.0",
    },
    globalNames: ["setInterval", "setTimeout"],
  };
}

function getUndiciShim(): Shim {
  return {
    package: {
      name: "undici",
      version: "^6.0.0",
    },
    globalNames: [
      "fetch",
      "File",
      "FormData",
      "Headers",
      "Request",
      "Response",
      typeOnly("BodyInit"),
      typeOnly("HeadersInit"),
      typeOnly("ReferrerPolicy"),
      typeOnly("RequestInit"),
      typeOnly("RequestCache"),
      typeOnly("RequestMode"),
      typeOnly("RequestRedirect"),
      typeOnly("ResponseInit"),
    ],
  };
}

function getDomExceptionShim(): Shim {
  return {
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
  };
}

function getWeakRefShim(): Shim {
  return {
    package: {
      name: "@deno/sham-weakref",
      version: "~0.1.0",
    },
    globalNames: ["WeakRef", typeOnly("WeakRefConstructor")],
  };
}

function getWebSocketShim(): Shim {
  return {
    package: {
      name: "ws",
      version: "^8.13.0",
    },
    typesPackage: {
      name: "@types/ws",
      version: "^8.5.4",
      peerDependency: false,
    },
    globalNames: [{
      name: "WebSocket",
      exportName: "default",
    }],
  };
}

function typeOnly(name: string): GlobalName {
  return {
    name,
    typeOnly: true,
  };
}
