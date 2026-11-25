import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema refuses undecodable media as a bad request", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      MediaDecodingFailed: {
        description: "the audio data is not valid base64",
        media: "Audio",
      },
    }),
  );

  deepStrictEqual(parsed, {
    done: true,
    error: {
      code: 400,
      description: "the audio data is not valid base64",
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
