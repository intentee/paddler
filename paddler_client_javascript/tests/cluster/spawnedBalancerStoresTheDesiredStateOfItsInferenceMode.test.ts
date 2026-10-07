import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { fetchJson } from "../../src/fetchJson";
import { BalancerDesiredStateSchema } from "../../src/schemas/BalancerDesiredState";
import { inferenceModes } from "../../src/schemas/InferenceMode";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

for (const inferenceMode of inferenceModes) {
  test(`spawned ${inferenceMode} balancer stores the desired state of its inference mode`, async function () {
    await withSpawnedBalancer(
      {
        bufferedRequestTimeoutMilliseconds:
          PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
        inferenceMode,
      },
      async function ({ management }) {
        const { inference_settings } = await fetchJson({
          schema: BalancerDesiredStateSchema,
          signal: new AbortController().signal,
          url: `http://${management}/api/v1/balancer_desired_state`,
        });

        deepStrictEqual(Object.keys(inference_settings), [inferenceMode]);
      },
    );
  });
}
