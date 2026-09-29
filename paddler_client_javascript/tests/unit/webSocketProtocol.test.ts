import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { webSocketProtocol } from "../src/webSocketProtocol";

test("https: maps to wss:", function () {
  strictEqual(webSocketProtocol("https:"), "wss:");
});

test("http: maps to ws:", function () {
  strictEqual(webSocketProtocol("http:"), "ws:");
});

test("anything other than https: maps to ws:", function () {
  strictEqual(webSocketProtocol("file:"), "ws:");
});
