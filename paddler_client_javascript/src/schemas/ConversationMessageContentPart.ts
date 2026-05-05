import { z } from "zod";

export const ConversationMessageContentPartSchema = z.union([
  z.object({
    type: z.literal("text"),
    text: z.string(),
  }),
  z.object({
    type: z.literal("image_url"),
    image_url: z.object({ url: z.string() }),
  }),
]);

export type ConversationMessageContentPart = z.infer<
  typeof ConversationMessageContentPartSchema
>;
