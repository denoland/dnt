// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

import { Client } from "./mod.ts";
import { assertEquals } from "https://deno.land/std@0.128.0/testing/asserts.ts";
import * as path from "https://deno.land/std@0.128.0/path/mod.ts";
import { isDeno } from "https://deno.land/x/which_runtime@0.2.0/mod.ts";

Deno.test("should get data from web socket server", async (t) => {
  const server = Deno.run({
    cmd: ["deno", "run", "-A", isDeno ? path.dirname(path.fromFileUrl(import.meta.url)) + "/../web_socket_server.ts" : "../../../web_socket_server.ts"],
  });

  await new Promise(resolve => setTimeout(resolve, 250));

  for (let i = 0; i < 2; i++) {
    await t.step(`attempt ${i + 1}`, async (t) => {
      const server = await Client.create();

      await t.step("should get values", async () => {
        assertEquals(await server.getValue(), "1");
        assertEquals(await server.getValue(), "2");
      });

      server.close();
    });
  }

  server.kill("SIGTERM");
  server.close();
});
