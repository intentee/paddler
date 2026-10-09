import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { AgentExitedBeforeReadyError } from "../AgentExitedBeforeReadyError";
import { agentsStreamUrl } from "../agentsStreamUrl";
import { CLAP_USAGE_ERROR_EXIT_CODE } from "../clapUsageErrorExitCode";
import { spawnAgent } from "../spawnAgent";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { waitForAgentReady } from "../waitForAgentReady";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const AGENT = { name: "agent-with-slots-it-rejects", slots: 0 };

test("waitForAgentReady reports an agent that exits", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      await rejects(
        waitForAgentReady({
          agentName: AGENT.name,
          agentProcess: spawnAgent({
            agent: AGENT,
            managementAddress: management,
          }),
          agentsStates: streamEventSource({
            schema: AgentsResponseSchema,
            url: agentsStreamUrl(management),
          }),
          expectedSlotsTotal: AGENT.slots,
        }),
        function (error: unknown) {
          return (
            error instanceof AgentExitedBeforeReadyError &&
            error.agentName === AGENT.name &&
            error.exitCode === CLAP_USAGE_ERROR_EXIT_CODE
          );
        },
      );
    },
  );
});
