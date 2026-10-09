import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";
import { lastValueFrom, toArray } from "rxjs";

import { fetchJson } from "../../src/fetchJson";
import { BalancerDesiredStateSchema } from "../../src/schemas/BalancerDesiredState";
import type { DecideParams } from "../../src/schemas/DecideParams";
import { InferenceServiceDecideResponseSchema } from "../../src/schemas/InferenceServiceDecideResponse";
import { streamHttpNdjson } from "../../src/streamHttpNdjson";
import { putBalancerDesiredState } from "../putBalancerDesiredState";
import { SHORT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../shortBufferedRequestTimeout";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const GATEWAY_TIMEOUT = 504;

test("streamHttpNdjson delivers a decision timeout without agents", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        SHORT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ inference, management }) {
      const storedDesiredState = await fetchJson({
        schema: BalancerDesiredStateSchema,
        signal: new AbortController().signal,
        url: `http://${management}/api/v1/balancer_desired_state`,
      });

      await putBalancerDesiredState({
        desiredState: { ...storedDesiredState, inference_mode: "Decision" },
        managementAddress: management,
      });

      const decideParams: DecideParams = {
        questions: [
          { id: "paid", instructions: "Was it paid?", options: ["no", "yes"] },
        ],
        state: "The invoice was paid on time.",
      };
      const responses = await lastValueFrom(
        streamHttpNdjson({
          body: decideParams,
          schema: InferenceServiceDecideResponseSchema,
          signal: new AbortController().signal,
          url: `http://${inference}/api/v1/decide`,
        }).pipe(toArray()),
      );

      deepStrictEqual(
        responses.map(function ({ done, error }) {
          return { done, errorCode: error?.code };
        }),
        [{ done: true, errorCode: GATEWAY_TIMEOUT }],
      );
    },
  );
});
