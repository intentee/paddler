import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { AgentsStreamClosedError } from "../AgentsStreamClosedError";
import { agentsStreamUrl } from "../agentsStreamUrl";
import {
  PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
  SpawnedBalancer,
} from "../SpawnedBalancer";
import { waitForAgentReady } from "../waitForAgentReady";
import { withReadyModelLessAgent } from "../withReadyModelLessAgent";

const AGENT = { name: "agent-that-never-gets-its-slots", slots: 1 };

test("waitForAgentReady reports a closed stream", async function () {
  const balancer = await SpawnedBalancer.spawn({
    bufferedRequestTimeoutMilliseconds:
      PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
  });

  await withReadyModelLessAgent(
    { agent: AGENT, managementAddress: balancer.addresses.management },
    async function (agentProcess) {
      const closedStreamRejection = rejects(
        waitForAgentReady({
          agentName: AGENT.name,
          agentProcess,
          agentsStates: streamEventSource({
            schema: AgentsResponseSchema,
            url: agentsStreamUrl(balancer.addresses.management),
          }),
          expectedSlotsTotal: AGENT.slots,
        }),
        function (error: unknown) {
          return (
            error instanceof AgentsStreamClosedError &&
            error.agentName === AGENT.name
          );
        },
      );

      await balancer.process.terminate();
      await closedStreamRejection;
    },
  );
});
