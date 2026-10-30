import { rejects } from "node:assert/strict";
import { test } from "node:test";

import { fetchJson } from "../../src/fetchJson";
import { HttpError } from "../../src/HttpError";
import { BalancerDesiredStateSchema } from "../../src/schemas/BalancerDesiredState";
import { putBalancerDesiredState } from "../putBalancerDesiredState";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const BAD_REQUEST = 400;

test("putBalancerDesiredState rejects an invalid state", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ management }) {
      const storedDesiredState = await fetchJson({
        schema: BalancerDesiredStateSchema,
        signal: new AbortController().signal,
        url: `http://${management}/api/v1/balancer_desired_state`,
      });

      await rejects(
        putBalancerDesiredState({
          desiredState: {
            ...storedDesiredState,
            model_runtime_parameters: {
              ...storedDesiredState.model_runtime_parameters,
              n_gpu_layers: -2,
            },
          },
          managementAddress: management,
        }),
        function (error: unknown) {
          return error instanceof HttpError && error.statusCode === BAD_REQUEST;
        },
      );
    },
  );
});
