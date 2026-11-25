import { z } from "zod";

export const SamplingParametersSchema = z
  .object({
    min_p: z.number().nullable(),
    penalty_frequency: z.number().nullable(),
    penalty_last_n: z.number().int().nullable(),
    penalty_presence: z.number().nullable(),
    penalty_repeat: z.number().nullable(),
    temperature: z.number().nullable(),
    top_k: z.number().int().nullable(),
    top_p: z.number().nullable(),
  })
  .strict();

export type SamplingParameters = z.infer<typeof SamplingParametersSchema>;
