// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

export class Client {
  #ws: WebSocket;

  constructor(ws: WebSocket) {
    this.#ws = ws;
  }

  static async create() {
    return new Promise<Client>((resolve, reject) => {
      const ws = new WebSocket("ws://localhost:8089");
      ws.onerror = (event) => {
        reject(event);
      };
      ws.onopen = () => {
        resolve(new Client(ws));
        // @ts-ignore: waiting on https://github.com/DefinitelyTyped/DefinitelyTyped/pull/59237
        ws.onerror = null;
      };
    });
  }

  close() {
    this.#ws.close();
  }

  getValue() {
    return new Promise((resolve, reject) => {
      this.#ws.onerror = ev => {
        reject(ev);
      };
      this.#ws.onmessage = ev => {
        resolve(ev.data);
        // @ts-ignore: waiting on https://github.com/DefinitelyTyped/DefinitelyTyped/pull/59237
        this.#ws.onmessage = null;
      };
      this.#ws.send("value");
    })
  }
}
