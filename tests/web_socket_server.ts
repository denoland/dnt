import { serve } from "https://deno.land/std@0.128.0/http/mod.ts";

const port = 8089;
serve(handleReq, { port });
console.log("Ready");

function handleReq(req: Request): Response {
  const upgrade = req.headers.get("upgrade") || "";
  if (upgrade.toLowerCase() !== "websocket") {
    return new Response("request isn't trying to upgrade to websocket.");
  }
  const { socket, response } = Deno.upgradeWebSocket(req);
  let value = 0;
  socket.onmessage = (e) => {
    console.error("socket message:", e.data);
    value++;
    socket.send(value.toString());
  };
  socket.onerror = (e) => {
    console.error("Had error:", (e as any).message);
  };
  return response;
}
