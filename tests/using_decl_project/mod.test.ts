import { DisposableClass } from "./mod.ts";

Deno.test("disposable", () => {
  const disposable = new DisposableClass();
  {
    using inner = disposable;
    if (inner.wasDisposed) {
      throw new Error("Failed.");
    }
  }
  if (!disposable.wasDisposed) {
    throw new Error("Failed.");
  }
});
