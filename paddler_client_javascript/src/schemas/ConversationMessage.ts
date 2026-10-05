import { z } from "zod";

import { ConversationMessageContentPartSchema } from "./ConversationMessageContentPart";

export const ConversationMessageSchema = z.object({
  role: z.string(),
  content: z.union([
    z.string(),
    z.array(ConversationMessageContentPartSchema),
  ]),
});

export type ConversationMessage = z.infer<typeof ConversationMessageSchema>;
