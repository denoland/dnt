// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import type { GlobalName, Shim } from "../transform.ts";

/** Provide `true` to use the shim in both the distributed and test code,
 * `"dev"` to only use it in the test code, or `false` to not use the shim
 * at all.
 *
 * @remarks These all default to `false`.
 */
export interface ShimOptions {
  /** Shim the `Deno` namespace. */
  deno?: boolean | "dev";
  /** Shim the global `setTimeout` and `setInterval` functions with
   * Deno and browser compatible versions.
   */
  timers?: boolean | "dev";
  /** Shim the global `confirm`, `alert`, and `prompt` functions. */
  prompts?: boolean | "dev";
  /** Shim the `Blob` global with the one from the `"buffer"` module. */
  blob?: boolean | "dev";
  /** Shim the `crypto` global. */
  crypto?: boolean | "dev";
  /** Shim `fetch`, `File`, `FormData`, `Headers`, `Request`, and `Response` by
   * using the "undici" package (https://www.npmjs.com/package/undici).
   */
  undici?: boolean | "dev";
  /** Custom shims to use. */
  custom?: Shim[];
  /** Custom shims to use only for the test code. */
  customDev?: Shim[];
}

export function shimOptionsToTransformShims(options: ShimOptions) {
  const shims: Shim[] = [];
  const testShims: Shim[] = [];

  add(options.deno, getDenoShim);
  add(options.blob, getBlobShim);
  add(options.crypto, getCryptoShim);
  add(options.prompts, getPromptsShim);
  add(options.timers, getTimersShim);
  add(options.undici, getUndiciShim);

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
      version: "~0.1.1",
    },
    globalNames: ["Deno"],
  };
}

function getCryptoShim(): Shim {
  return {
    package: {
      name: "@deno/shim-crypto",
      version: "~0.2.0",
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
    package: {
      name: "buffer",
    },
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
      version: "^4.12.1",
    },
    globalNames: [
      "fetch",
      "File",
      "FormData",
      "Headers",
      "Request",
      "Response",
    ],
  };
}

function typeOnly(name: string): GlobalName {
  return {
    name,
    typeOnly: true,
  };
}
