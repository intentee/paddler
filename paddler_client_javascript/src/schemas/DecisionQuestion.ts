import { z } from "zod";

export const DecisionQuestionSchema = z
  .object({
    id: z.string(),
    instructions: z.string(),
    options: z.array(z.string()).min(1),
  })
  .strict();

export type DecisionQuestion = z.infer<typeof DecisionQuestionSchema>;
