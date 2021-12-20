import jsonData from "./data.json" assert { type: "json" };

export function getOutput() {
  return jsonData.prop;
}

export async function getDynamicOutput() {
  const module = await import("./data.json", { assert: { type: "json" } });
  return module.default.prop;
}
