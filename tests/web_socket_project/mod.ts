// Copyright 2018-2024 the Deno authors. MIT license.

// This code isn't bullet proof and is lazily designed
// for the purpose of the test. Don't use it elsewhere.
export class Client {
  #ws: WebSocket;

  constructor(ws: WebSocket) {
    this.#ws = ws;
  }

  static create() {
    return new Promise<Client>((resolve, reject) => {
      const ws = new WebSocket("ws://localhost:8089");
      ws.onerror = (e) => {
        reject((e as any).message);
      };
      ws.onopen = () => {
        resolve(new Client(ws));
        ws.onerror = null;
        ws.onopen = null;
      };
    });
  }

  close() {
    // Attempt to prevent left over ops.
    return new Promise<void>((resolve, reject) => {
      this.#ws.onerror = (e) => {
        reject((e as any).message);
      };
      this.#ws.onclose = () => resolve();
      this.#ws.close();
    });
  }

  getValue() {
    return new Promise((resolve, reject) => {
      this.#ws.onerror = (ev) => {
        reject(ev);
      };
      this.#ws.onmessage = (ev) => {
        resolve(ev.data);
        // @ts-ignore: waiting on https://github.com/DefinitelyTyped/DefinitelyTyped/pull/59237
        this.#ws.onmessage = null;
      };
      this.#ws.send("value");
    });
  }
}
