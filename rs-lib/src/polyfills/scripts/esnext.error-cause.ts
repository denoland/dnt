declare global {
  interface Error {
    cause?: Error;
  }
}

export {};
