declare global {
  interface Error {
    cause?: unknown;
  }
}

export {};
