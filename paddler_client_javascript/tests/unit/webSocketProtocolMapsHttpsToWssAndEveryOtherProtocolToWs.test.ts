import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { webSocketProtocol } from "../../src/webSocketProtocol";

const protocolMappings = [
  { expectedWebSocketProtocol: "wss:", httpProtocol: "https:" },
  { expectedWebSocketProtocol: "ws:", httpProtocol: "http:" },
  { expectedWebSocketProtocol: "ws:", httpProtocol: "file:" },
] as const;

test("webSocketProtocol maps https: to wss: and every other protocol to ws:", function () {
  for (const { expectedWebSocketProtocol, httpProtocol } of protocolMappings) {
    strictEqual(
      webSocketProtocol(httpProtocol),
      expectedWebSocketProtocol,
      httpProtocol,
    );
  }
});
