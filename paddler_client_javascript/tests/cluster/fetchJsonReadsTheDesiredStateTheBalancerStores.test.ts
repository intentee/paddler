import { test } from "node:test";

import { assertFetchJsonReadsStoredInferenceParameters } from "../assertFetchJsonReadsStoredInferenceParameters";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

test("fetchJson reads the desired state the balancer stores", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      await assertFetchJsonReadsStoredInferenceParameters({
        inferenceParameters: { enable_embeddings: true },
        managementAddress: management,
      });
    },
  );
});
