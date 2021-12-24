import { createMergeProxy } from "./createMergeProxy.ts";

Deno.test("should merge two objects", () => {
  const baseObj = {
    shared1: "base_shared1",
    base1: "base_base1",
  };
  const extObj = {
    shared1: "ext_shared1",
    ext1: "ext_ext1",
  };
  const merged = createMergeProxy(baseObj, extObj);

  // get
  assertEqual(merged.base1, "base_base1");
  assertEqual(merged.shared1, "ext_shared1");
  assertEqual(merged.ext1, "ext_ext1");

  // keys
  const keys = Object.keys(merged);
  assertEqual(keys.length, 3);
  assertEqual(keys[0], "base1");
  assertEqual(keys[1], "shared1");
  assertEqual(keys[2], "ext1");

  // has own
  assertEqual(Object.hasOwn(merged, "ext1"), true);
  assertEqual(Object.hasOwn(merged, "base1"), true);
  assertEqual(Object.hasOwn(merged, "random"), false);

  // setting property
  merged.ext1 = "asdf";
  assertEqual(merged.ext1, "asdf");
  assertEqual((baseObj as any).ext1, "asdf");
  assertEqual(extObj.ext1, undefined);

  // deleting property
  delete (merged as any).shared1;
  assertEqual(merged.shared1, undefined);
  assertEqual(baseObj.shared1, undefined);
  assertEqual(extObj.shared1, undefined);
});

Deno.test("should allow spreading globalThis", () => {
  const extObj = {
    test: 5,
  };
  const merged = createMergeProxy(globalThis, extObj);
  // was getting an error when not using Reflect.ownKeys
  const _result = { ...merged, extObj };
  const { test } = merged;
  assertEqual(test, 5);
});

function assertEqual(a: any, b: any) {
  if (a !== b) {
    throw new Error(`The value ${a} did not equal ${b}.`);
  }
}
