import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { GeneratedToolCallSchema } from "../../../src/schemas/GeneratedToolCall";

test("GeneratedToolCallSchema keeps the inner JSON of ValidJson arguments intact", function () {
  const parsed = GeneratedToolCallSchema.parse({
    id: "call_0",
    index: 0,
    name: "get_weather",
    arguments: { ValidJson: { location: "Paris" } },
  });

  strictEqual(parsed.id, "call_0");
  strictEqual(parsed.name, "get_weather");
  deepStrictEqual(parsed.arguments, { ValidJson: { location: "Paris" } });
});
