import { z } from "zod";

import { AgentSchema, type Agent } from "./Agent";

export const AgentsResponseSchema = z
  .object({
    agents: z.array(AgentSchema),
  })
  .strict()
  .transform(function ({ agents }) {
    return Object.freeze({
      agents: agents.sort(function (a: Agent, b: Agent) {
        return String(a.name).localeCompare(String(b.name));
      }),
    });
  });

export type AgentsResponse = z.infer<typeof AgentsResponseSchema>;
