import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { AgentsSnapshotUndeserializableError } from "../AgentsSnapshotUndeserializableError";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { waitForAgentReady } from "../waitForAgentReady";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

test("waitForAgentReady reports an undeserializable snapshot", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      await rejects(
        waitForAgentReady({
          agentName: "agent-that-never-joined",
          agentsStates: streamEventSource({
            schema: AgentsResponseSchema,
            url: `http://${management}/api/v1/buffered_requests/stream`,
          }),
          expectedSlotsTotal: 1,
        }),
        AgentsSnapshotUndeserializableError,
      );
    },
  );
});
