import { z } from "zod";

export const DecisionQuestionSchema = z.discriminatedUnion("type", [
  z
    .object({
      criteria: z.record(z.string(), z.json()),
      instructions: z.json().optional(),
      type: z.literal("choice"),
    })
    .strict(),
  z
    .object({
      criteria: z.record(z.string(), z.json()).optional(),
      instructions: z.json().optional(),
      type: z.literal("noul"),
    })
    .strict(),
  z
    .object({
      criteria: z.array(z.json()),
      instructions: z.json().optional(),
      type: z.literal("score"),
    })
    .strict(),
]);

export type DecisionQuestion = z.infer<typeof DecisionQuestionSchema>;
