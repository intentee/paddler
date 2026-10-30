import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { ContinueFromConversationHistoryParamsSchema } from "../../../src/schemas/ContinueFromConversationHistoryParams";
import { continueFromConversationHistoryParamsWithoutTools } from "./continueFromConversationHistoryParamsWithoutTools";

test("ContinueFromConversationHistoryParamsSchema accepts tool call parsing with a declared tool", function () {
  strictEqual(
    ContinueFromConversationHistoryParamsSchema.safeParse({
      ...continueFromConversationHistoryParamsWithoutTools,
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
