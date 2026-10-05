import { rejects } from "node:assert/strict";
import { test } from "node:test";
import { isDeepStrictEqual } from "node:util";

import { fetchJson } from "../../src/fetchJson";
import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { BalancerDesiredStateSchema } from "../../src/schemas/BalancerDesiredState";
import { streamEventSource } from "../../src/streamEventSource";
import { AgentReportedIssuesError } from "../AgentReportedIssuesError";
import { putBalancerDesiredState } from "../putBalancerDesiredState";
import { spawnAgent } from "../spawnAgent";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { waitForAgentReady } from "../waitForAgentReady";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const AGENT = { name: "agent-without-its-model", slots: 1 };

test("waitForAgentReady reports agent issues", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      const storedDesiredState = await fetchJson({
        schema: BalancerDesiredStateSchema,
        signal: new AbortController().signal,
        url: `http://${management}/api/v1/balancer_desired_state`,
      });

      await putBalancerDesiredState({
        desiredState: {
          ...storedDesiredState,
          model: { LocalToAgent: "/nonexistent/model.gguf" },
        },
        managementAddress: management,
      });

      const agentProcess = spawnAgent({
        agent: AGENT,
        managementAddress: management,
      });

      try {
        await rejects(
          waitForAgentReady({
            agentName: AGENT.name,
            agentsStates: streamEventSource({
              schema: AgentsResponseSchema,
              url: `http://${management}/api/v1/agents/stream`,
            }),
            expectedSlotsTotal: AGENT.slots,
          }),
          function (error: unknown) {
            return (
              error instanceof AgentReportedIssuesError &&
              isDeepStrictEqual(
                error.issues.map(function (issue) {
                  return Object.keys(issue);
                }),
                [["ModelFileDoesNotExist"]],
              )
            );
          },
        );
      } finally {
        await agentProcess.terminate();
      }
    },
  );
});
