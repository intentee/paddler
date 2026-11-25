import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema refuses an unsupported conversation feature as a bad request", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      ConversationFeatureNotSupported: {
        description: "the chat template of the model does not support thinking",
        feature: "Thinking",
      },
    }),
  );

  deepStrictEqual(parsed, {
    done: true,
    error: {
      code: 400,
      description: "the chat template of the model does not support thinking",
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
