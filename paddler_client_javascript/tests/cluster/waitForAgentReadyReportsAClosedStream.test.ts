import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { AgentsStreamClosedError } from "../AgentsStreamClosedError";
import { waitForAgentReady } from "../waitForAgentReady";
import { withConnectionClosingListener } from "../withConnectionClosingListener";

test("waitForAgentReady reports a closed stream", async function () {
  await withConnectionClosingListener(async function (address) {
    await rejects(
      waitForAgentReady({
        agentName: "agent-that-never-joined",
        agentsStates: streamEventSource({
          schema: AgentsResponseSchema,
          url: `http://${address}/api/v1/agents/stream`,
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
});
