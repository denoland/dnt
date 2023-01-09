// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
import { assertEquals } from "./test.deps.ts";

export function wildcardAssertEquals(actual: string, expected: string) {
  const parts = expected.split("[WILDCARD]");
  let index = 0;
  for (const part of parts) {
    if (part.length === 0) {
      continue;
    }

    if (index === 0) {
      assertEquals(actual.substring(0, part.length), part);
      index = part.length;
    } else {
      let foundIndex = undefined;
      while (true) {
        const nextIndex = actual.indexOf(part, (foundIndex ?? index) + 1);
        if (nextIndex === -1) {
          break;
        } else {
          foundIndex = nextIndex;
        }
      }
      if (foundIndex == null) {
        throw new Error(`Could not find part: ${part}`);
      }
      index = foundIndex + part.length;
    }
  }
  if (index !== actual.length && parts[parts.length - 1].length > 0) {
    throw new Error(
      `Text was missing end of text. ${index} -- ${actual.length}`,
    );
  }
}
