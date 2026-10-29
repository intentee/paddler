import { z } from "zod";

import { AgentDecisionsServingSchema } from "./AgentDecisionsServing";

export const AgentRuntimeStatusSchema = z.union([
  z.literal("Idle"),
  z
    .object({
      Serving: z
        .object({
          decisions: AgentDecisionsServingSchema,
          slots_total: z.number(),
        })
        .strict(),
    })
    .strict(),
]);

export type AgentRuntimeStatus = z.infer<typeof AgentRuntimeStatusSchema>;
