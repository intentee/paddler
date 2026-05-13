import { z } from "zod";

export const GrammarConstraintSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("gbnf"),
    grammar: z.string(),
    root: z.string(),
  }),
  z.object({
    type: z.literal("json_schema"),
    schema: z.string(),
  }),
]);

export type GrammarConstraint = z.infer<typeof GrammarConstraintSchema>;
