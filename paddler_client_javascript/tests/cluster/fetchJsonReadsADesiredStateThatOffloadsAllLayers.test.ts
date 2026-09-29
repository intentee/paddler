import { test } from "node:test";

import { assertFetchJsonReadsStoredInferenceParameters } from "../assertFetchJsonReadsStoredInferenceParameters";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const OFFLOAD_ALL_LAYERS = -1;

test("fetchJson reads a desired state that offloads all layers", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      await assertFetchJsonReadsStoredInferenceParameters({
        inferenceParameters: { n_gpu_layers: OFFLOAD_ALL_LAYERS },
        managementAddress: management,
      });
    },
  );
});
