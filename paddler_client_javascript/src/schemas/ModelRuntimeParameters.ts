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

const I32_MAX = 2_147_483_647;

export const ModelRuntimeParametersSchema = z
  .object({
    context_size: z.number().int().min(1),
    k_cache_dtype: z.enum(cacheDtypes),
    n_batch: z.number().int().min(1).max(I32_MAX),
    n_gpu_layers: z.number().int().min(-1),
    v_cache_dtype: z.enum(cacheDtypes),
  })
  .strict()
  .superRefine(function (parameters, context) {
    if (parameters.n_batch > parameters.context_size) {
      context.addIssue({
        code: "custom",
        message: "n_batch must not exceed context_size",
        path: ["n_batch"],
      });
    }
  });

export type ModelRuntimeParameters = z.infer<
  typeof ModelRuntimeParametersSchema
>;
