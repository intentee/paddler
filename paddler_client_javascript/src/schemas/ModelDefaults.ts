import { z } from "zod";

export const ModelDefaultsSchema = z
  .object({
    context_size: z.number(),
    sampling: z
      .object({
        min_p: z.number(),
        penalty_frequency: z.number(),
        penalty_last_n: z.number(),
        penalty_presence: z.number(),
        penalty_repeat: z.number(),
        temperature: z.number(),
        top_k: z.number(),
        top_p: z.number(),
      })
      .strict(),
  })
  .strict();

export type ModelDefaults = z.infer<typeof ModelDefaultsSchema>;
