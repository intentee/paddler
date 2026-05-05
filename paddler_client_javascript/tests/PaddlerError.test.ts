import test from "ava";

import { ConnectionDroppedError } from "../src/ConnectionDroppedError";
import { HttpError } from "../src/HttpError";
import { JsonError } from "../src/JsonError";
import { PaddlerError } from "../src/PaddlerError";
import { ServerError } from "../src/ServerError";
import { WebSocketError } from "../src/WebSocketError";

test("HttpError extends PaddlerError and carries the status code", function (t) {
  const err = new HttpError(503, "Service Unavailable");

  t.true(err instanceof PaddlerError);
  t.true(err instanceof Error);
  t.is(err.statusCode, 503);
  t.is(err.message, "Service Unavailable");
  t.is(err.name, "HttpError");
});

test("JsonError carries raw payload alongside its message", function (t) {
  const err = new JsonError("unexpected token", "{not-json");

  t.true(err instanceof PaddlerError);
  t.is(err.raw, "{not-json");
  t.is(err.name, "JsonError");
});

test("WebSocketError is a distinct subclass", function (t) {
  const err = new WebSocketError("socket closed");

  t.true(err instanceof PaddlerError);
  t.is(err.name, "WebSocketError");
});

test("ConnectionDroppedError carries the request id", function (t) {
  const err = new ConnectionDroppedError("req-1");

  t.true(err instanceof PaddlerError);
  t.is(err.requestId, "req-1");
  t.true(err.message.includes("req-1"));
});

test("ServerError carries an integer code", function (t) {
  const err = new ServerError(429, "rate limit");

  t.true(err instanceof PaddlerError);
  t.is(err.code, 429);
  t.is(err.message, "rate limit");
});
