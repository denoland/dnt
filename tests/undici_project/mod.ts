// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

export async function getJson(url: string) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(response.statusText);
  }
  return response.json();
}
