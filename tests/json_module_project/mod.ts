import jsonData from "./data.json" with { type: "json" };

export function getOutput() {
  return jsonData.prop;
}

export async function getDynamicOutput() {
  const module = await import("./data.json", { with: { type: "json" } });
  return module.default.prop;
}
