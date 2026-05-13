import { deepStrictEqual, strictEqual, throws } from "node:assert/strict";
import { test } from "node:test";

import { ParsedToolCallSchema } from "../../src/schemas/ParsedToolCall";

test("ValidJson arguments parse with the inner JSON kept intact", function () {
  const parsed = ParsedToolCallSchema.parse({
    id: "call_0",
    name: "get_weather",
    arguments: { ValidJson: { location: "Paris" } },
  });

  strictEqual(parsed.id, "call_0");
  strictEqual(parsed.name, "get_weather");
  deepStrictEqual(parsed.arguments, { ValidJson: { location: "Paris" } });
});

test("InvalidJson arguments preserve the raw string", function () {
  const parsed = ParsedToolCallSchema.parse({
    id: "call_1",
    name: "get_weather",
    arguments: { InvalidJson: "not json" },
  });

  deepStrictEqual(parsed.arguments, { InvalidJson: "not json" });
});

test("rejects payloads missing the discriminated arguments wrapper", function () {
  throws(function () {
    ParsedToolCallSchema.parse({
      id: "call_2",
      name: "get_weather",
      arguments: { location: "Paris" },
    });
  });
});
