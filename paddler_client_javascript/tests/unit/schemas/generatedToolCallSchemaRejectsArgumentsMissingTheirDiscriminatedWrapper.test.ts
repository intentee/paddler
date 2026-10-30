import { throws } from "node:assert/strict";
import { test } from "node:test";

import { GeneratedToolCallSchema } from "../../../src/schemas/GeneratedToolCall";

test("GeneratedToolCallSchema rejects arguments missing their discriminated wrapper", function () {
  throws(function () {
    GeneratedToolCallSchema.parse({
      id: "call_2",
      index: 2,
      name: "get_weather",
      arguments: { location: "Paris" },
    });
  });
});
