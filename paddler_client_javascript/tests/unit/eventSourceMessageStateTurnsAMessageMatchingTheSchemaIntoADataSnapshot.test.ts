import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { eventSourceMessageState } from "../../src/eventSourceMessageState";
import { CountSchema } from "./CountSchema";

test("eventSourceMessageState turns a message matching the schema into a data snapshot", function () {
  deepStrictEqual(
    eventSourceMessageState({ data: '{"count":7}', schema: CountSchema }),
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
