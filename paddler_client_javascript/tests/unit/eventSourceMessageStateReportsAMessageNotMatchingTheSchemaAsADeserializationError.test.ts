import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { eventSourceDeserializationErrorState } from "../../src/EventSourceDeserializationErrorState";
import { eventSourceMessageState } from "../../src/eventSourceMessageState";
import { CountSchema } from "./CountSchema";

test("eventSourceMessageState reports a message not matching the schema as a deserialization error", function () {
  strictEqual(
    eventSourceMessageState({ data: '{"count":"seven"}', schema: CountSchema }),
    eventSourceDeserializationErrorState,
  );
});
