import { z } from "zod";

export const EmbeddingsUnsupportedReasonSchema = z.enum([
  "ModelDeclaresNoVectorPooling",
]);

export type EmbeddingsUnsupportedReason = z.infer<
  typeof EmbeddingsUnsupportedReasonSchema
>;
