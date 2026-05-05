import { z } from "zod";

import { GrammarConstraintSchema } from "./GrammarConstraint";

export const ContinueFromRawPromptParamsSchema = z
  .object({
    grammar: GrammarConstraintSchema.nullable().optional(),
    max_tokens: z.number().int(),
    raw_prompt: z.string(),
  })
  .strict();

export type ContinueFromRawPromptParams = z.infer<
  typeof ContinueFromRawPromptParamsSchema
>;
