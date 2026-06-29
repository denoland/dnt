// Copyright 2018-2024 the Deno authors. MIT license.

export function other() {
  type test1 = typeof globalThis.fetch;
  type test2 = typeof globalThis;
  type test3 = test2["fetch"];
  return fetch;
}

export async function getCryptoKeyPair(
  keyUsages: KeyUsage[],
): Promise<globalThis.CryptoKeyPair> {
  const keyPair = await crypto.subtle.generateKey(
    {
      name: "RSA-OAEP",
      modulusLength: 4096,
      publicExponent: new Uint8Array([1, 0, 1]),
      hash: "SHA-256",
    },
    true,
    keyUsages,
  );
  return keyPair;
}

export function throwDomException() {
  throw new DOMException("My message", "Something");
}

export function localShimValue() {
  return new ArrayBuffer(5);
}
