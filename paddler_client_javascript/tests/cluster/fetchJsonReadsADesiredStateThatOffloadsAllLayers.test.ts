import { test } from "node:test";

import { ALL_GPU_LAYERS } from "../allGpuLayers";
import { assertFetchJsonReadsStoredDesiredState } from "../assertFetchJsonReadsStoredDesiredState";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

test("fetchJson reads a desired state that offloads all layers", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      await assertFetchJsonReadsStoredDesiredState({
        managementAddress: management,
        updateDesiredState(storedDesiredState) {
          return {
            ...storedDesiredState,
            model_runtime_parameters: {
              ...storedDesiredState.model_runtime_parameters,
              n_gpu_layers: ALL_GPU_LAYERS,
            },
          };
        },
      });
    },
  );
});
