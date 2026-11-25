import { z } from "zod";

export const AgentDecisionsServingSchema = z.enum([
  "DecisionModelNotServed",
  "NotServed",
  "Served",
]);

export type AgentDecisionsServing = z.infer<typeof AgentDecisionsServingSchema>;
