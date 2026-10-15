import { z } from "zod";

export const SamplingParametersSchema = z
  .object({
    min_p: z.number(),
    penalty_frequency: z.number(),
    penalty_last_n: z.number().int(),
    penalty_presence: z.number(),
    penalty_repeat: z.number(),
    temperature: z.number(),
    top_k: z.number().int(),
    top_p: z.number(),
  })
  .strict();

export type SamplingParameters = z.infer<typeof SamplingParametersSchema>;
