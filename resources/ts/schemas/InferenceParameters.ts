import { z } from "zod";

export const poolingTypes = [
  "Cls",
  "Last",
  "Mean",
  "None",
  "Rank",
  "Unspecified",
] as const;

export const InferenceParametersSchema = z
  .object({
    batch_n_tokens: z.number(),
    context_size: z.number(),
    embedding_n_seq_max: z.number(),
    enable_embeddings: z.boolean(),
    min_p: z.number(),
    penalty_frequency: z.number(),
    penalty_last_n: z.number(),
    penalty_presence: z.number(),
    penalty_repeat: z.number(),
    pooling_type: z.enum(poolingTypes),
    temperature: z.number(),
    top_k: z.number(),
    top_p: z.number(),
  })
  .strict();

export type InferenceParameters = z.infer<typeof InferenceParametersSchema>;

export type BooleanKeys = {
  [K in keyof InferenceParameters]: InferenceParameters[K] extends boolean
    ? K
    : never;
}[keyof InferenceParameters];
export type NumberKeys = {
  [K in keyof InferenceParameters]: InferenceParameters[K] extends number
    ? K
    : never;
}[keyof InferenceParameters];
