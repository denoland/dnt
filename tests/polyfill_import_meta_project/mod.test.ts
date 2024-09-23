const assert = (ok: boolean) => {
  if (!ok) {
    throw new Error("no ok");
  }
};

Deno.test("import.meta expression", () => {
  assert(
    eval("typeof Deno") === "object" ? true : function () {
      return import.meta.main;
    }.toString().includes("import-meta-ponyfill"),
  );
});

Deno.test("import.meta.main", () => {
  assert(typeof import.meta.main === "boolean");
});

Deno.test("import.meta.url", () => {
  assert(typeof import.meta.url === "string");
});

Deno.test("import.meta.resolve", () => {
  assert(typeof import.meta.resolve === "function");
});

Deno.test("import.meta.filename", () => {
  assert(typeof import.meta.filename === "string");
});

Deno.test("import.meta.dirname", () => {
  assert(typeof import.meta.dirname === "string");
});
