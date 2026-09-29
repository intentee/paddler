import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { AgentsStreamClosedError } from "../AgentsStreamClosedError";
import {
  PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
  SpawnedBalancer,
} from "../SpawnedBalancer";
import { waitForAgentReady } from "../waitForAgentReady";

test("waitForAgentReady reports a closed stream", async function () {
  const balancer = await SpawnedBalancer.spawn({
    bufferedRequestTimeoutMilliseconds:
      PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
  });

  await balancer.process.terminate();

  await rejects(
    waitForAgentReady({
      agentName: "agent-that-never-joined",
      agentsStates: streamEventSource({
        schema: AgentsResponseSchema,
        url: `http://${balancer.addresses.management}/api/v1/agents/stream`,
      }),
      expectedSlotsTotal: 1,
    }),
    function (error: unknown) {
      return (
        error instanceof AgentsStreamClosedError &&
        error.agentName === "agent-that-never-joined"
      );
    },
  );
});
