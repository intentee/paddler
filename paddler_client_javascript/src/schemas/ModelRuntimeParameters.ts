import { z } from "zod";

export const cacheDtypes = [
  "F32",
  "F16",
  "BF16",
  "Q8_0",
  "Q4_0",
  "Q4_1",
  "IQ4_NL",
  "Q5_0",
  "Q5_1",
] as const;

export const ModelRuntimeParametersSchema = z
  .object({
    context_size: z.number().int(),
    k_cache_dtype: z.enum(cacheDtypes),
    n_batch: z.number().int(),
    n_gpu_layers: z.number().int(),
    v_cache_dtype: z.enum(cacheDtypes),
  })
  .strict();

export type ModelRuntimeParameters = z.infer<
  typeof ModelRuntimeParametersSchema
>;
