import { throws } from "node:assert/strict";
import { test } from "node:test";

import { fetchJson } from "../../src/fetchJson";
import { BalancerDesiredStateSchema } from "../../src/schemas/BalancerDesiredState";
import { DesiredStateServesAnotherModeError } from "../DesiredStateServesAnotherModeError";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";
import { withTextGenerationSettings } from "../withTextGenerationSettings";

test("withTextGenerationSettings refuses a desired state of another mode", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
      inferenceMode: "Embeddings",
    },
    async function ({ management }) {
      const embeddingsDesiredState = await fetchJson({
        schema: BalancerDesiredStateSchema,
        signal: new AbortController().signal,
        url: `http://${management}/api/v1/balancer_desired_state`,
      });

      throws(function () {
        withTextGenerationSettings(embeddingsDesiredState, structuredClone);
      }, DesiredStateServesAnotherModeError);
    },
  );
});
