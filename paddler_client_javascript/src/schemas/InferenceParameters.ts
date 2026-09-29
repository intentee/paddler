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

export const poolingTypes = [
  "Cls",
  "Last",
  "Mean",
  "None",
  "Rank",
  "Unspecified",
] as const;

const I32_MAX = 2_147_483_647;
const NEUTRAL_PENALTY_REPEAT = 1;

const ProbabilitySchema = z.number().min(0).max(1);

export const InferenceParametersSchema = z
  .object({
    n_batch: z.number().int().min(1).max(I32_MAX),
    context_size: z.number().int().min(1),
    embedding_batch_size: z.number().int().min(1),
    enable_embeddings: z.boolean(),
    image_resize_to_fit: z.number().int().min(1),
    k_cache_dtype: z.enum(cacheDtypes),
    v_cache_dtype: z.enum(cacheDtypes),
    min_p: ProbabilitySchema,
    n_gpu_layers: z.number().int().min(-1),
    penalty_frequency: z.number(),
    penalty_last_n: z.number().int().min(0),
    penalty_presence: z.number(),
    penalty_repeat: z.number().gt(0),
    pooling_type: z.enum(poolingTypes),
    temperature: z.number().min(0),
    top_k: z.number().int().min(0),
    top_p: ProbabilitySchema,
  })
  .strict()
  .superRefine(function (parameters, context) {
    const penaltiesAreNeutral =
      parameters.penalty_repeat === NEUTRAL_PENALTY_REPEAT &&
      parameters.penalty_frequency === 0 &&
      parameters.penalty_presence === 0;

    if (parameters.n_batch > parameters.context_size) {
      context.addIssue({
        code: "custom",
        message: "n_batch must not exceed context_size",
        path: ["n_batch"],
      });
    }

    if (parameters.penalty_last_n === 0 && !penaltiesAreNeutral) {
      context.addIssue({
        code: "custom",
        message: "penalty strengths require a penalty_last_n window",
        path: ["penalty_last_n"],
      });
    }

    if (parameters.penalty_last_n > 0 && penaltiesAreNeutral) {
      context.addIssue({
        code: "custom",
        message: "penalty_last_n requires at least one penalty strength",
        path: ["penalty_last_n"],
      });
    }
  });

export type InferenceParameters = z.infer<typeof InferenceParametersSchema>;
