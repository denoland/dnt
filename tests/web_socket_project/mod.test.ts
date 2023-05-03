// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

import { Client } from "./mod.ts";
import { assertEquals } from "https://deno.land/std@0.182.0/testing/asserts.ts";
import * as path from "https://deno.land/std@0.182.0/path/mod.ts";
import { isDeno } from "https://deno.land/x/which_runtime@0.2.0/mod.ts";

Deno.test("should get data from web socket server", async (t) => {
  const server = new Deno.Command("deno", {
    args: [
      "run",
      "-A",
      isDeno
        ? path.dirname(path.fromFileUrl(import.meta.url)) +
          "/../web_socket_server.ts"
        : "../../../web_socket_server.ts",
    ],
    stdout: "piped",
  });
  const child = server.spawn();

  // wait for some output from the server
  const stdout = child.stdout.getReader({ mode: "byob" });
  await stdout.read(new Uint8Array(1));

  for (let i = 0; i < 2; i++) {
    await t.step(`attempt ${i + 1}`, async (t) => {
      const server = await Client.create();

      await t.step("should get values", async () => {
        assertEquals(await server.getValue(), "1");
        assertEquals(await server.getValue(), "2");
      });

      await server.close();
    });
  }

  child.kill("SIGTERM");
});
