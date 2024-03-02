const port = 8089;
Deno.serve({ port }, handleReq);
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
