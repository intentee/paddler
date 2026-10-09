import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { once } from "node:events";
import { test } from "node:test";
import { firstValueFrom, lastValueFrom, toArray } from "rxjs";

import { fetchJson } from "../../src/fetchJson";
import { inferenceSocketClient } from "../../src/inferenceSocketClient";
import { BalancerDesiredStateSchema } from "../../src/schemas/BalancerDesiredState";
import { inferenceSocketUrl } from "../inferenceSocketUrl";
import { putBalancerDesiredState } from "../putBalancerDesiredState";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const SERVICE_UNAVAILABLE = 503;

test("inferenceSocketClient reports the cluster inference mode", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function (addresses) {
      const storedDesiredState = await fetchJson({
        schema: BalancerDesiredStateSchema,
        signal: new AbortController().signal,
        url: `http://${addresses.management}/api/v1/balancer_desired_state`,
      });

      await putBalancerDesiredState({
        desiredState: { ...storedDesiredState, inference_mode: "Embeddings" },
        managementAddress: addresses.management,
      });

      const webSocket = new WebSocket(inferenceSocketUrl(addresses));

      try {
        const { clusterInferenceMode$, continueConversation } =
          inferenceSocketClient({ webSocket });
        const firstClusterInferenceMode = firstValueFrom(clusterInferenceMode$);

        await once(webSocket, "open");

        strictEqual(await firstClusterInferenceMode, "Embeddings");

        const responses = await lastValueFrom(
          continueConversation({
            enableThinking: false,
            messages: [{ content: "Hello", role: "user" }],
          }).pipe(toArray()),
        );

        deepStrictEqual(
          responses.map(function ({ done, error }) {
            return { done, errorCode: error?.code };
          }),
          [{ done: true, errorCode: SERVICE_UNAVAILABLE }],
        );
      } finally {
        webSocket.close();
      }
    },
  );
});
