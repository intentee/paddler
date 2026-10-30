import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { eventSourceDeserializationErrorState } from "../../src/EventSourceDeserializationErrorState";
import { eventSourceMessageState } from "../../src/eventSourceMessageState";
import { CountSchema } from "./CountSchema";

test("eventSourceMessageState reports a message that is not JSON as a deserialization error", function () {
  strictEqual(
    eventSourceMessageState({ data: "not json", schema: CountSchema }),
    eventSourceDeserializationErrorState,
  );
});
