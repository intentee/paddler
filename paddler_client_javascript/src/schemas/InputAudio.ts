import { z } from "zod";

export const InputAudioSchema = z
  .object({
    data: z.string(),
    format: z.enum(["mp3", "wav"]),
  })
  .strict();

export type InputAudio = z.infer<typeof InputAudioSchema>;
