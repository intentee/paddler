import test from "ava";

import { webSocketProtocol } from "../src/webSocketProtocol";

test("https: maps to wss:", function (t) {
  t.is(webSocketProtocol("https:"), "wss:");
});

test("http: maps to ws:", function (t) {
  t.is(webSocketProtocol("http:"), "ws:");
});

test("anything other than https: maps to ws:", function (t) {
  t.is(webSocketProtocol("file:"), "ws:");
});
