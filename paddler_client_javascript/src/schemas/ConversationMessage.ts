import { z } from "zod";

import { ConversationMessageContentPartSchema } from "./ConversationMessageContentPart";
import { ConversationToolCallSchema } from "./ConversationToolCall";

const ConversationMessageContentSchema = z.union([
  z.string(),
  z.array(ConversationMessageContentPartSchema),
]);

export const ConversationMessageSchema = z.union([
  z.strictObject({
    AssistantReasoning: z.strictObject({
      reasoning_content: z.string(),
    }),
  }),
  z.strictObject({
    AssistantToolCalls: z.strictObject({
      content: ConversationMessageContentSchema,
      tool_calls: z.array(ConversationToolCallSchema),
    }),
  }),
  z.strictObject({
    Message: z.strictObject({
      content: ConversationMessageContentSchema,
      role: z.string(),
    }),
  }),
  z.strictObject({
    ToolResult: z.strictObject({
      content: ConversationMessageContentSchema,
      tool_call_id: z.string(),
    }),
  }),
]);

export type ConversationMessage = z.infer<typeof ConversationMessageSchema>;
