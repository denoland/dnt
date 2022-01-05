// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

export function addAsync(a: number, b: number) {
  return new Promise<number>((resolve, reject) => {
    // The shim should be injected here because setTimeout and setInterval
    // return `Timeout` in node.js, but we want them to return `number`
    const timeoutResult: number = setTimeout(
      () => reject(new Error("fail")),
      50,
    );
    const intervalResult: number = setInterval(
      () => reject(new Error("fail")),
      50,
    );
    clearTimeout(timeoutResult);
    clearInterval(intervalResult);

    setTimeout(() => {
      resolve(a + b);
    }, 100);
  });
}

export function other() {
  return fetch;
}

export async function getCryptoKeyPair(
  keyUsages: KeyUsage[],
): Promise<CryptoKeyPair> {
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
