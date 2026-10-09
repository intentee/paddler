import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { AgentsSnapshotUndeserializableError } from "../AgentsSnapshotUndeserializableError";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { waitForAgentReady } from "../waitForAgentReady";
import { withReadyModelLessAgent } from "../withReadyModelLessAgent";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const AGENT = { name: "agent-awaited-on-the-wrong-stream", slots: 1 };

test("waitForAgentReady reports an undeserializable snapshot", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      await withReadyModelLessAgent(
        { agent: AGENT, managementAddress: management },
        async function (agentProcess) {
          await rejects(
            waitForAgentReady({
              agentName: AGENT.name,
              agentProcess,
              agentsStates: streamEventSource({
                schema: AgentsResponseSchema,
                url: `http://${management}/api/v1/buffered_requests/stream`,
              }),
              expectedSlotsTotal: AGENT.slots,
            }),
            AgentsSnapshotUndeserializableError,
          );
        },
      );
    },
  );
});
