import { deepEqual } from "node:assert/strict";
import { test } from "node:test";

import { ConversationMessageSchema } from "../../../src/schemas/ConversationMessage";

test("ConversationMessageSchema accepts a tool call turn and its tool result", function () {
  const toolCallTurn = {
    AssistantToolCalls: {
      content: "",
      tool_calls: [
        {
          arguments: { location: "Paris" },
          id: "call_weather",
          name: "get_weather",
        },
      ],
    },
  };
  const toolResult = {
    ToolResult: { content: "sunny", tool_call_id: "call_weather" },
  };

  deepEqual(
    [toolCallTurn, toolResult].map(function (message) {
      return ConversationMessageSchema.parse(message);
    }),
    [toolCallTurn, toolResult],
  );
});
