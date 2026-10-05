import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { fetchJson } from "../../src/fetchJson";
import { AgentsResponseSchema } from "../../src/schemas/AgentsResponse";
import { withPaddlerCluster } from "../withPaddlerCluster";

test("withPaddlerCluster registers a model-less agent", async function () {
  const agents = await withPaddlerCluster(
    {
      agents: [{ name: "model-less-agent", slots: 1 }],
      configureDesiredState(storedDesiredState) {
        return storedDesiredState;
      },
    },
    async function ({ management }) {
      return (
        await fetchJson({
          schema: AgentsResponseSchema,
          signal: new AbortController().signal,
          url: `http://${management}/api/v1/agents`,
        })
      ).agents;
    },
  );

  deepStrictEqual(
    agents.map(function ({ name }) {
      return name;
    }),
    ["model-less-agent"],
  );
});
