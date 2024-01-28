// https://github.com/denoland/deno/blob/7fc5bfe51b7d405aaa5293ec6f1a8f1e9119aea2/cli/dts/lib.esnext.array.d.ts
declare global {
  interface Array<T> {
    /**
     * Returns the value of the last element in the array where predicate is true, and undefined
     * otherwise.
     * @param predicate find calls predicate once for each element of the array, in ascending
     * order, until it finds one where predicate returns true. If such an element is found, find
     * immediately returns that element value. Otherwise, find returns undefined.
     * @param thisArg If provided, it will be used as the this value for each invocation of
     * predicate. If it is not provided, undefined is used instead.
     */
    findLast<S extends T>(
      predicate: (this: void, value: T, index: number, obj: T[]) => value is S,
      thisArg?: any,
    ): S | undefined;
    findLast(
      predicate: (value: T, index: number, obj: T[]) => unknown,
      thisArg?: any,
    ): T | undefined;

    /**
     * Returns the index of the last element in the array where predicate is true, and -1
     * otherwise.
     * @param predicate find calls predicate once for each element of the array, in ascending
     * order, until it finds one where predicate returns true. If such an element is found,
     * findIndex immediately returns that element index. Otherwise, findIndex returns -1.
     * @param thisArg If provided, it will be used as the this value for each invocation of
     * predicate. If it is not provided, undefined is used instead.
     */
    findLastIndex(
      predicate: (value: T, index: number, obj: T[]) => unknown,
      thisArg?: any,
    ): number;
  }
  interface Uint8Array {
    /**
     * Returns the value of the last element in the array where predicate is true, and undefined
     * otherwise.
     * @param predicate findLast calls predicate once for each element of the array, in descending
     * order, until it finds one where predicate returns true. If such an element is found, findLast
     * immediately returns that element value. Otherwise, findLast returns undefined.
     * @param thisArg If provided, it will be used as the this value for each invocation of
     * predicate. If it is not provided, undefined is used instead.
     */
    findLast<S extends number>(
      predicate: (
          value: number,
          index: number,
          array: Uint8Array,
      ) => value is S,
      thisArg?: any,
    ): S | undefined;
    findLast(
        predicate: (value: number, index: number, array: Uint8Array) => unknown,
        thisArg?: any,
    ): number | undefined;

    /**
     * Returns the index of the last element in the array where predicate is true, and -1
     * otherwise.
     * @param predicate findLastIndex calls predicate once for each element of the array, in descending
     * order, until it finds one where predicate returns true. If such an element is found,
     * findLastIndex immediately returns that element index. Otherwise, findLastIndex returns -1.
     * @param thisArg If provided, it will be used as the this value for each invocation of
     * predicate. If it is not provided, undefined is used instead.
     */
    findLastIndex(
        predicate: (value: number, index: number, array: Uint8Array) => unknown,
        thisArg?: any,
    ): number;
  }
}

function findLastIndex(self: any, callbackfn: any, that: any) {
  const boundFunc = that === undefined ? callbackfn : callbackfn.bind(that);
  let index = self.length - 1;
  while (index >= 0) {
    const result = boundFunc(self[index], index, self);
    if (result) {
      return index;
    }
    index--;
  }
  return -1;
}

function findLast(self: any, callbackfn: any, that: any) {
  const index = self.findLastIndex(callbackfn, that);
  return index === -1 ? undefined : self[index];
}

if (!Array.prototype.findLastIndex) {
  Array.prototype.findLastIndex = function (callbackfn: any, that: any) {
    return findLastIndex(this, callbackfn, that);
  };
}

if (!Array.prototype.findLast) {
  Array.prototype.findLast = function (callbackfn: any, that: any) {
    return findLast(this, callbackfn, that);
  };
}

if (!Uint8Array.prototype.findLastIndex) {
  Uint8Array.prototype.findLastIndex = function (callbackfn: any, that: any) {
    return findLastIndex(this, callbackfn, that);
  };
}

if (!Uint8Array.prototype.findLast) {
  Uint8Array.prototype.findLast = function (callbackfn: any, that: any) {
    return findLast(this, callbackfn, that);
  };
}

export {};
