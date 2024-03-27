// Copyright 2018-2024 the Deno authors. MIT license.

export async function getJson(url: string) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(response.statusText);
  }
  return response.json();
}
