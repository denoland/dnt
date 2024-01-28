export function fromAsync<T>(generator: AsyncIterable<T>): Promise<T[]> {
  return Array.fromAsync(generator);
}
