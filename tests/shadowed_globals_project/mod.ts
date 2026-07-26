// Copyright 2018-2024 the Deno authors. MIT license.

// these declarations shadow identifiers that the emit relies on
// (ex. `Object.defineProperty(exports, "__esModule", { value: true });`)
const Object = "hello";

export class Symbol {
  text = `${Object} world`;
}

export function getText() {
  return new Symbol().text;
}

export default Object;
