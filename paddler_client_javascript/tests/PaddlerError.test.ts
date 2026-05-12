import { ok, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { ConnectionDroppedError } from "../src/ConnectionDroppedError";
import { HttpError } from "../src/HttpError";
import { JsonError } from "../src/JsonError";
import { PaddlerError } from "../src/PaddlerError";
import { ServerError } from "../src/ServerError";
import { WebSocketError } from "../src/WebSocketError";

test("HttpError extends PaddlerError and carries the status code", function () {
  const err = new HttpError(503, "Service Unavailable");

  ok(err instanceof PaddlerError);
  ok(err instanceof Error);
  strictEqual(err.statusCode, 503);
  strictEqual(err.message, "Service Unavailable");
  strictEqual(err.name, "HttpError");
});

test("JsonError carries raw payload alongside its message", function () {
  const err = new JsonError("unexpected token", "{not-json");

  ok(err instanceof PaddlerError);
  strictEqual(err.raw, "{not-json");
  strictEqual(err.name, "JsonError");
});

test("WebSocketError is a distinct subclass", function () {
  const err = new WebSocketError("socket closed");

  ok(err instanceof PaddlerError);
  strictEqual(err.name, "WebSocketError");
});

test("ConnectionDroppedError carries the request id", function () {
  const err = new ConnectionDroppedError("req-1");

  ok(err instanceof PaddlerError);
  strictEqual(err.requestId, "req-1");
  ok(err.message.includes("req-1"));
});

test("ServerError carries an integer code", function () {
  const err = new ServerError(429, "rate limit");

  ok(err instanceof PaddlerError);
  strictEqual(err.code, 429);
  strictEqual(err.message, "rate limit");
});
