import { z } from "zod";

export const ConversationToolCallSchema = z.strictObject({
  arguments: z.record(z.string(), z.unknown()),
  id: z.string(),
  name: z.string(),
});

export type ConversationToolCall = z.infer<typeof ConversationToolCallSchema>;
