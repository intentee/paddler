import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";
import { lastValueFrom, toArray } from "rxjs";

import { InferenceServiceGenerateTokensResponseSchema } from "../../src/schemas/InferenceServiceGenerateTokensResponse";
import { streamHttpNdjson } from "../../src/streamHttpNdjson";
import { SHORT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS } from "../shortBufferedRequestTimeout";
import { withSpawnedBalancer } from "../withSpawnedBalancer";

const GATEWAY_TIMEOUT = 504;

test("streamHttpNdjson delivers a timeout without agents", async function () {
  await withSpawnedBalancer(
    {
      bufferedRequestTimeoutMilliseconds:
        SHORT_BUFFERED_REQUEST_TIMEOUT_MILLISECONDS,
    },
    async function ({ inference }) {
      const responses = await lastValueFrom(
        streamHttpNdjson({
          body: {
            add_generation_prompt: true,
            conversation_history: [{ content: "Hello", role: "user" }],
            enable_thinking: false,
            max_tokens: 1,
          },
          schema: InferenceServiceGenerateTokensResponseSchema,
          signal: new AbortController().signal,
          url: `http://${inference}/api/v1/continue_from_conversation_history`,
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
