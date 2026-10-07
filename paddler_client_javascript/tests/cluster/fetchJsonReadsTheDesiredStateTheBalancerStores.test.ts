import { test } from "node:test";

import { assertFetchJsonReadsStoredDesiredState } from "../assertFetchJsonReadsStoredDesiredState";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";
import { withTextGenerationSettings } from "../withTextGenerationSettings";

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
          return withTextGenerationSettings(
            storedDesiredState,
            function (textGenerationSettings) {
              return {
                ...textGenerationSettings,
                use_chat_template_override: true,
              };
            },
          );
        },
      });
    },
  );
});
