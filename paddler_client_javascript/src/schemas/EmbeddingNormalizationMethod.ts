import { z } from "zod";

export const EmbeddingNormalizationMethodSchema = z.union([
  z.literal("L2"),
  z.literal("None"),
  z.object({
    RmsNorm: z.object({
      epsilon: z.number(),
    }),
  }),
]);

export type EmbeddingNormalizationMethod = z.infer<
  typeof EmbeddingNormalizationMethodSchema
>;
