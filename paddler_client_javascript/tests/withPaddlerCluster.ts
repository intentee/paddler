import { fetchJson } from "../src/fetchJson";
import { AgentsResponseSchema } from "../src/schemas/AgentsResponse";
import {
  BalancerDesiredStateSchema,
  type BalancerDesiredState,
} from "../src/schemas/BalancerDesiredState";
import { streamEventSource } from "../src/streamEventSource";
import type { AgentSpec } from "./AgentSpec";
import { agentsStreamUrl } from "./agentsStreamUrl";
import type { BalancerAddresses } from "./BalancerAddresses";
import { putBalancerDesiredState } from "./putBalancerDesiredState";
import { spawnAgent } from "./spawnAgent";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "./SpawnedBalancer";
import type { SpawnedProcess } from "./SpawnedProcess";
import { waitForAgentReady } from "./waitForAgentReady";
import { withSpawnedBalancer } from "./withSpawnedBalancer";

export function withPaddlerCluster<TResult>(
  {
    agents,
    configureDesiredState,
  }: {
    agents: ReadonlyArray<AgentSpec>;
    configureDesiredState(
      storedDesiredState: BalancerDesiredState,
    ): BalancerDesiredState;
  },
  body: (addresses: BalancerAddresses) => Promise<TResult>,
): Promise<TResult> {
  return withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function (addresses) {
      const managementAddress = addresses.management;
      const agentProcesses: SpawnedProcess[] = [];

      try {
        const desiredState = configureDesiredState(
          await fetchJson({
            schema: BalancerDesiredStateSchema,
            signal: new AbortController().signal,
            url: `http://${managementAddress}/api/v1/balancer_desired_state`,
          }),
        );

        await putBalancerDesiredState({ desiredState, managementAddress });

        for (const agent of agents) {
          const agentProcess = spawnAgent({ agent, managementAddress });

          agentProcesses.push(agentProcess);

          await waitForAgentReady({
            agentName: agent.name,
            agentProcess,
            agentsStates: streamEventSource({
              schema: AgentsResponseSchema,
              url: agentsStreamUrl(managementAddress),
            }),
            expectedSlotsTotal: desiredState.model === "None" ? 0 : agent.slots,
          });
        }

        return await body(addresses);
      } finally {
        for (const agentProcess of agentProcesses) {
          await agentProcess.terminate();
        }
      }
    },
  );
}
