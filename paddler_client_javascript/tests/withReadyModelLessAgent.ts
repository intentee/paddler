import { AgentsResponseSchema } from "../src/schemas/AgentsResponse";
import { streamEventSource } from "../src/streamEventSource";
import type { AgentSpec } from "./AgentSpec";
import { agentsStreamUrl } from "./agentsStreamUrl";
import { spawnAgent } from "./spawnAgent";
import type { SpawnedProcess } from "./SpawnedProcess";
import { waitForAgentReady } from "./waitForAgentReady";

const MODEL_LESS_SLOTS_TOTAL = 0;

export async function withReadyModelLessAgent<TResult>(
  {
    agent,
    managementAddress,
  }: {
    agent: AgentSpec;
    managementAddress: string;
  },
  body: (agentProcess: SpawnedProcess) => Promise<TResult>,
): Promise<TResult> {
  const agentProcess = spawnAgent({ agent, managementAddress });

  try {
    await waitForAgentReady({
      agentName: agent.name,
      agentProcess,
      agentsStates: streamEventSource({
        schema: AgentsResponseSchema,
        url: agentsStreamUrl(managementAddress),
      }),
      expectedSlotsTotal: MODEL_LESS_SLOTS_TOTAL,
    });

    return await body(agentProcess);
  } finally {
    await agentProcess.terminate();
  }
}
