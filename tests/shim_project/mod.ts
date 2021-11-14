// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

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
