import { test } from "node:test";

import { ContinueFromConversationHistoryParamsSchema } from "../../../src/schemas/ContinueFromConversationHistoryParams";
import { assertSchemaRejectsPaths } from "../../assertSchemaRejectsPaths";
import { continueFromConversationHistoryParamsWithoutTools } from "./continueFromConversationHistoryParamsWithoutTools";

test("ContinueFromConversationHistoryParamsSchema rejects tool call parsing without tools", function () {
  assertSchemaRejectsPaths({
    expectedPaths: [["parse_tool_calls"]],
    input: continueFromConversationHistoryParamsWithoutTools,
    schema: ContinueFromConversationHistoryParamsSchema,
  });
});
