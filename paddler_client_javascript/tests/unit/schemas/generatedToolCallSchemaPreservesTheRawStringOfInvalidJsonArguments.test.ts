import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { GeneratedToolCallSchema } from "../../../src/schemas/GeneratedToolCall";

test("GeneratedToolCallSchema preserves the raw string of InvalidJson arguments", function () {
  const parsed = GeneratedToolCallSchema.parse({
    id: "call_1",
    index: 1,
    name: "get_weather",
    arguments: { InvalidJson: "not json" },
  });

  deepStrictEqual(parsed.arguments, { InvalidJson: "not json" });
});
