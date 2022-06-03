export function findLast<T>(
  items: T[],
  predicate: (value: T, index: number, obj: T[]) => unknown,
  thisArg?: any,
): T | undefined;
export function findLast<T, S extends T>(
  items: T[],
  predicate: (this: void, value: T, index: number, obj: T[]) => value is S,
  thisArg?: any,
): T | undefined;
export function findLast<T>(
  items: T[],
  predicate: (value: T, index: number, obj: T[]) => unknown,
  thisArg?: any,
): T | undefined {
  const index = items.findLastIndex(predicate, thisArg);
  const value = items.findLast(predicate, thisArg);
  if (items[index] !== value) {
    throw new Error(
      `The returned value ${value} did not equal the element at index ${index} (${
        items[index]
      }).`,
    );
  }
  return value;
}
