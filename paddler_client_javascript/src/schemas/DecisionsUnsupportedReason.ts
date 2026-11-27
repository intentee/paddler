import { z } from "zod";

export const DecisionsUnsupportedReasonSchema = z.enum([
  "DecisionTypeUnsupportedByLlamaCpp",
  "ModelIsNotADecisionModel",
]);

export type DecisionsUnsupportedReason = z.infer<
  typeof DecisionsUnsupportedReasonSchema
>;
