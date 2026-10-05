import { z } from "zod";

import { ConversationMessageSchema } from "./ConversationMessage";
import { GrammarConstraintSchema } from "./GrammarConstraint";
import { ToolSchema } from "./Tool";

export const ContinueFromConversationHistoryParamsSchema = z
  .object({
    add_generation_prompt: z.boolean(),
    conversation_history: z.array(ConversationMessageSchema),
    enable_thinking: z.boolean(),
    grammar: GrammarConstraintSchema.nullable().optional(),
    max_tokens: z.number().int(),
    parse_tool_calls: z.boolean().optional(),
    tools: z.array(ToolSchema).optional(),
  })
  .strict();

export type ContinueFromConversationHistoryParams = z.infer<
  typeof ContinueFromConversationHistoryParamsSchema
>;
