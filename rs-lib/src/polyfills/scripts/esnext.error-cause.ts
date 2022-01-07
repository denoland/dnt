declare global {
  interface Error {
    cause?: any;
  }
}

export {};
