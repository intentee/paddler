import { z } from "zod";

export const poolingTypes = [
  "Cls",
  "Last",
  "Mean",
  "None",
  "Rank",
  "Unspecified",
] as const;

export const EmbeddingParametersSchema = z
  .object({
    embedding_batch_size: z.number().int().min(1),
    pooling_type: z.enum(poolingTypes),
  })
  .strict();

export type EmbeddingParameters = z.infer<typeof EmbeddingParametersSchema>;
