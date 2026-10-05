import { test } from "node:test";

import { ALL_GPU_LAYERS } from "../allGpuLayers";
import { assertFetchJsonReadsStoredInferenceParameters } from "../assertFetchJsonReadsStoredInferenceParameters";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

test("fetchJson reads a desired state that offloads all layers", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      await assertFetchJsonReadsStoredInferenceParameters({
        inferenceParameters: { n_gpu_layers: ALL_GPU_LAYERS },
        managementAddress: management,
      });
    },
  );
});
