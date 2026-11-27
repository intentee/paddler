import { z } from "zod";

export const TextGenerationUnsupportedReasonSchema = z.enum([
  "AttentionIsNotCausal",
  "EncoderDecoderArchitecture",
  "NoTextOutput",
]);

export type TextGenerationUnsupportedReason = z.infer<
  typeof TextGenerationUnsupportedReasonSchema
>;
