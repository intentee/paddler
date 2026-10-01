import { z } from "zod";

import { AgentSchema, type Agent } from "./Agent";

export const AgentsResponseSchema = z
  .object({
    agents: z.array(AgentSchema),
  })
  .strict()
  .transform(function ({ agents }) {
    return Object.freeze({
      agents: agents.sort(function (left: Agent, right: Agent) {
        return (left.name ?? left.id).localeCompare(right.name ?? right.id);
      }),
    });
  });

export type AgentsResponse = z.infer<typeof AgentsResponseSchema>;
