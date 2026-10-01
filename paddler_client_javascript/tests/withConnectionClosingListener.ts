import { ok } from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:net";

export async function withConnectionClosingListener<TResult>(
  body: (address: string) => Promise<TResult>,
): Promise<TResult> {
  const server = createServer(function (socket) {
    socket.destroy();
  });

  server.listen(0, "127.0.0.1");

  await once(server, "listening");

  const listenedAddress = server.address();

  try {
    ok(typeof listenedAddress === "object" && listenedAddress !== null);

    return await body(`${listenedAddress.address}:${listenedAddress.port}`);
  } finally {
    server.close();
  }
}
