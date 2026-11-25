import { z } from "zod";

export const ConversationUnsupportedReasonSchema = z.enum([
  "ModelGeneratesNoText",
  "ModelHasNoChatTemplate",
]);

export type ConversationUnsupportedReason = z.infer<
  typeof ConversationUnsupportedReasonSchema
>;
