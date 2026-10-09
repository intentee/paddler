import { z } from "zod";

import { InferenceModeSchema } from "./InferenceMode";

export const AgentRuntimeStatusSchema = z.union([
  z.literal("Idle"),
  z
    .object({
      Serving: z
        .object({
          inference_mode: InferenceModeSchema,
          slots_total: z.number(),
        })
        .strict(),
    })
    .strict(),
]);

export type AgentRuntimeStatus = z.infer<typeof AgentRuntimeStatusSchema>;
