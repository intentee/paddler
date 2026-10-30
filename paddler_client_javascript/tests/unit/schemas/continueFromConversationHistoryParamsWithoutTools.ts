import type { ContinueFromConversationHistoryParams } from "../../../src/schemas/ContinueFromConversationHistoryParams";

export const continueFromConversationHistoryParamsWithoutTools: ContinueFromConversationHistoryParams =
  {
    add_generation_prompt: true,
    conversation_history: [{ Message: { content: "Hello!", role: "user" } }],
    enable_thinking: false,
    max_tokens: 100,
    parse_tool_calls: true,
  };
