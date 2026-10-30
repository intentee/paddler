import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema carries parsed tool calls without ending the stream", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      ToolCallParsed: [
        {
          arguments: { ValidJson: { location: "Paris" } },
          id: "call_42",
          index: 42,
          name: "get_weather",
        },
      ],
    }),
  );

  strictEqual(parsed.done, false);
  deepStrictEqual(parsed.toolCalls, [
    {
      arguments: { ValidJson: { location: "Paris" } },
      id: "call_42",
      index: 42,
      name: "get_weather",
    },
  ]);
});
