import { z } from "zod";

import { ModelCapabilitiesSchema } from "./ModelCapabilities";

export const AgentRuntimeStatusSchema = z.union([
  z.literal("Idle"),
  z
    .object({
      Serving: z
        .object({
          capabilities: ModelCapabilitiesSchema,
          slots_total: z.number(),
        })
        .strict(),
    })
    .strict(),
]);

export type AgentRuntimeStatus = z.infer<typeof AgentRuntimeStatusSchema>;
