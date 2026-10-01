import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";
import { filter, firstValueFrom, lastValueFrom, toArray } from "rxjs";

import { BufferedRequestsResponseSchema } from "../../src/schemas/BufferedRequestsResponse";
import { InferenceServiceGenerateTokensResponseSchema } from "../../src/schemas/InferenceServiceGenerateTokensResponse";
import { streamEventSource } from "../../src/streamEventSource";
import { streamHttpNdjson } from "../../src/streamHttpNdjson";
import { PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../SpawnedBalancer";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

test("streamHttpNdjson completes when its request is aborted", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        PADDLER_DEFAULT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ inference, management }) {
      const abortController = new AbortController();
      const responses = lastValueFrom(
        streamHttpNdjson({
          body: {
            add_generation_prompt: true,
            conversation_history: [{ content: "Hello", role: "user" }],
            enable_thinking: false,
            max_tokens: 1,
          },
          schema: InferenceServiceGenerateTokensResponseSchema,
          signal: abortController.signal,
          url: `http://${inference}/api/v1/continue_from_conversation_history`,
        }).pipe(toArray()),
      );

      await firstValueFrom(
        streamEventSource({
          schema: BufferedRequestsResponseSchema,
          url: `http://${management}/api/v1/buffered_requests/stream`,
        }).pipe(
          filter(function (bufferedRequestsState) {
            return (
              bufferedRequestsState.isOk &&
              bufferedRequestsState.data.buffered_requests_current === 1
            );
          }),
        ),
      );

      abortController.abort();

      deepStrictEqual(await responses, []);
    },
  );
});
