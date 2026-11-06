import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { GeneratedToolCallSchema } from "../../../src/schemas/GeneratedToolCall";

test("GeneratedToolCallSchema preserves the raw string and parse error of InvalidJson arguments", function () {
  const invalidJson = {
    error: {
      column: 1,
      line: 1,
      message: "expected ident at line 1 column 1",
    },
    raw: "not json",
  };
  const parsed = GeneratedToolCallSchema.parse({
    id: "call_1",
    index: 1,
    name: "get_weather",
    arguments: { InvalidJson: invalidJson },
  });

  deepStrictEqual(parsed.arguments, { InvalidJson: invalidJson });
});
