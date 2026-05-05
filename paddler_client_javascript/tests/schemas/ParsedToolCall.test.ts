import test from "ava";

import { ParsedToolCallSchema } from "../../src/schemas/ParsedToolCall";

test("ValidJson arguments parse with the inner JSON kept intact", function (t) {
  const parsed = ParsedToolCallSchema.parse({
    id: "call_0",
    name: "get_weather",
    arguments: { ValidJson: { location: "Paris" } },
  });

  t.is(parsed.id, "call_0");
  t.is(parsed.name, "get_weather");
  t.deepEqual(parsed.arguments, { ValidJson: { location: "Paris" } });
});

test("InvalidJson arguments preserve the raw string", function (t) {
  const parsed = ParsedToolCallSchema.parse({
    id: "call_1",
    name: "get_weather",
    arguments: { InvalidJson: "not json" },
  });

  t.deepEqual(parsed.arguments, { InvalidJson: "not json" });
});

test("rejects payloads missing the discriminated arguments wrapper", function (t) {
  t.throws(function () {
    ParsedToolCallSchema.parse({
      id: "call_2",
      name: "get_weather",
      arguments: { location: "Paris" },
    });
  });
});
