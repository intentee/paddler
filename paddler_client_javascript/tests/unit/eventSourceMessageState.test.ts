import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";
import { z } from "zod";

import { eventSourceDeserializationErrorState } from "../../src/EventSourceDeserializationErrorState";
import { eventSourceMessageState } from "../../src/eventSourceMessageState";

const Schema = z.object({ count: z.number() });

test("a message matching the schema becomes a data snapshot", function () {
  deepStrictEqual(
    eventSourceMessageState({ data: '{"count":7}', schema: Schema }),
    {
      data: { count: 7 },
      isConnected: true,
      isConnectionError: false,
      isDeserializationError: false,
      isInitial: false,
      isOk: true,
    },
  );
});

test("a message that is not JSON is a deserialization error", function () {
  strictEqual(
    eventSourceMessageState({ data: "not json", schema: Schema }),
    eventSourceDeserializationErrorState,
  );
});

test("a message that does not match the schema is a deserialization error", function () {
  strictEqual(
    eventSourceMessageState({ data: '{"count":"seven"}', schema: Schema }),
    eventSourceDeserializationErrorState,
  );
});
