import { z } from "zod";

const NEUTRAL_PENALTY_REPEAT = 1;

const ProbabilitySchema = z.number().min(0).max(1);

export const SamplingParametersSchema = z
  .object({
    min_p: ProbabilitySchema,
    penalty_frequency: z.number(),
    penalty_last_n: z.number().int().min(0),
    penalty_presence: z.number(),
    penalty_repeat: z.number().gt(0),
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

export type SamplingParameters = z.infer<typeof SamplingParametersSchema>;
