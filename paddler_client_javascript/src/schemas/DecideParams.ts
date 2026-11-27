import { z } from "zod";

import { DecisionQuestionSchema } from "./DecisionQuestion";

export const DecideParamsSchema = z
  .object({
    images: z.array(z.string()).optional(),
    questions: z.record(z.string(), DecisionQuestionSchema).refine(function (
      questions,
    ) {
      return Object.keys(questions).length > 0;
    }, "a decision needs at least one question"),
    state: z.json(),
  })
  .strict();

export type DecideParams = z.infer<typeof DecideParamsSchema>;
