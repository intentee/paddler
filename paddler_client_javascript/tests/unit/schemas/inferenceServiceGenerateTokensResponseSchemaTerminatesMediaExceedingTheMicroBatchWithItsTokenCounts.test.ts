import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema terminates media exceeding the micro batch with its token counts", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      MediaExceedsMicroBatch: {
        media_tokens: 368,
        micro_batch_tokens: 100,
      },
    }),
  );

  deepStrictEqual(parsed, {
    done: true,
    error: {
      code: 400,
      description:
        "media required 368 tokens but one agent micro batch holds 100 tokens",
    },
    generated_by: null,
    ok: false,
    rawToolCallTokens: null,
    request_id: "req-1",
    summary: null,
    token: null,
    tokenKind: null,
    toolCalls: null,
  });
});
