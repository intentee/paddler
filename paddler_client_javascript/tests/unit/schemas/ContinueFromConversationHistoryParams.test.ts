import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import {
  ContinueFromConversationHistoryParamsSchema,
  type ContinueFromConversationHistoryParams,
} from "../../../src/schemas/ContinueFromConversationHistoryParams";
import { assertSchemaRejectsPaths } from "../../assertSchemaRejectsPaths";

const paramsWithoutTools: ContinueFromConversationHistoryParams = {
  add_generation_prompt: true,
  conversation_history: [{ content: "Hello!", role: "user" }],
  enable_thinking: false,
  max_tokens: 100,
  parse_tool_calls: true,
};

test("accepts tool call parsing with a declared tool", function () {
  strictEqual(
    ContinueFromConversationHistoryParamsSchema.safeParse({
      ...paramsWithoutTools,
      tools: [
        {
          function: {
            description: "Get the current weather for a location",
            name: "get_weather",
          },
          type: "function",
        },
      ],
    }).success,
    true,
  );
});

test("rejects tool call parsing without tools", function () {
  assertSchemaRejectsPaths({
    expectedPaths: [["parse_tool_calls"]],
    input: paramsWithoutTools,
    schema: ContinueFromConversationHistoryParamsSchema,
  });
});
