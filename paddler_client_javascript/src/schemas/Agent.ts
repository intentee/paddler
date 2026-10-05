import { z } from "zod";

import { AgentStatusSchema } from "./AgentStatus";

export const AgentSchema = z
  .object({
    id: z.string(),
    name: z.string().nullable(),
    slots_processing: z.number(),
    status: AgentStatusSchema,
  })
  .strict();

export type Agent = z.infer<typeof AgentSchema>;
