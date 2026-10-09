import { test } from "node:test";

import { assertFetchJsonReadsStoredDesiredState } from "../assertFetchJsonReadsStoredDesiredState";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

test("fetchJson reads the desired state the balancer stores", async function () {
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
            inference_mode: "Decision",
            text_generation: {
              ...storedDesiredState.text_generation,
              use_chat_template_override: true,
            },
          };
        },
      });
    },
  );
});
