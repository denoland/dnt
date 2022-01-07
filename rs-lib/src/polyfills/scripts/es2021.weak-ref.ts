declare global {
  // https://github.com/microsoft/TypeScript/blob/main/src/lib/es2021.weakref.d.ts

  interface WeakRef<T extends object> {
    readonly [Symbol.toStringTag]: "WeakRef";

    /**
     * Returns the WeakRef instance's target object, or undefined if the target object has been
     * reclaimed.
     */
    deref(): T | undefined;
  }

  interface WeakRefConstructor {
    readonly prototype: WeakRef<any>;

    /**
     * Creates a WeakRef instance for the given target object.
     * @param target The target object for the WeakRef instance.
     */
    new <T extends object>(target: T): WeakRef<T>;
  }

  var WeakRef: WeakRefConstructor;
}

if (globalThis.WeakRef == null) {
  globalThis.WeakRef = class WeakRef<T extends object> {
    readonly [Symbol.toStringTag]: "WeakRef";

    constructor(_target: T) {
    }

    deref(): T | undefined {
      throw new Error("WeakRef is not supported in Node 14 and below.")
    }
  };
}

export {};
