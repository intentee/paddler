import { z } from "zod";

import { DecisionQuestionSchema } from "./DecisionQuestion";

export const DecideParamsSchema = z
  .object({
    questions: z.array(DecisionQuestionSchema).min(1),
    state: z.string(),
  })
  .strict();

export type DecideParams = z.infer<typeof DecideParamsSchema>;
