// this project must only use the `Promise.withResolvers` polyfill so that it
// ends up alone in the generated polyfill file (see #440)
export function withResolvers<T>() {
  return Promise.withResolvers<T>();
}
