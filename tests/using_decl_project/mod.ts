(Symbol as any).dispose ??= Symbol("dispose");

export class DisposableClass {
  wasDisposed = false;

  [Symbol.dispose]() {
    this.wasDisposed = true;
  }
}
