import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { fetchJson } from "../../src/fetchJson";
import { BalancerDesiredStateSchema } from "../../src/schemas/BalancerDesiredState";
import { putBalancerDesiredState } from "../putBalancerDesiredState";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

test("fetchJson reads the desired state the balancer stores", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      const url = `http://${management}/api/v1/balancer_desired_state`;
      const storedDesiredState = await fetchJson({
        schema: BalancerDesiredStateSchema,
        signal: new AbortController().signal,
        url,
      });
      const embeddingsDesiredState = {
        ...storedDesiredState,
        inference_parameters: {
          ...storedDesiredState.inference_parameters,
          enable_embeddings: true,
        },
      };

      await putBalancerDesiredState({
        desiredState: embeddingsDesiredState,
        managementAddress: management,
      });

      deepStrictEqual(
        await fetchJson({
          schema: BalancerDesiredStateSchema,
          signal: new AbortController().signal,
          url,
        }),
        embeddingsDesiredState,
      );
    },
  );
});
