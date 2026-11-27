import { z } from "zod";

import { InputAudioSchema } from "./InputAudio";

export const ConversationMessageContentPartSchema = z.union([
  z.object({
    type: z.literal("text"),
    text: z.string(),
  }),
  z.object({
    type: z.literal("image_url"),
    image_url: z.object({ url: z.string() }),
  }),
  z.object({
    type: z.literal("input_audio"),
    input_audio: InputAudioSchema,
  }),
]);

export type ConversationMessageContentPart = z.infer<
  typeof ConversationMessageContentPartSchema
>;
