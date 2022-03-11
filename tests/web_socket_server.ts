import { serve } from "https://deno.land/std@0.128.0/http/mod.ts";

serve(handleReq, { port: 8089 });

function handleReq(req: Request): Response {
  const upgrade = req.headers.get("upgrade") || "";
  if (upgrade.toLowerCase() !== "websocket") {
    return new Response("request isn't trying to upgrade to websocket.");
  }
  const { socket, response } = Deno.upgradeWebSocket(req);
  let value = 0;
  socket.onmessage = (e) => {
    console.log("socket message:", e.data);
    value++;
    socket.send(value.toString());
  };
  return response;
}
