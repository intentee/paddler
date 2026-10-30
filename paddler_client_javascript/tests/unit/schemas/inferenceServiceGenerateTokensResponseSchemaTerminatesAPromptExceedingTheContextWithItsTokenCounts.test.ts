import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema terminates a prompt exceeding the context with its token counts", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      PromptExceedsContextSize: {
        prompt_tokens: 9895,
        sequence_context_size: 8192,
      },
    }),
  );

  deepStrictEqual(parsed, {
    done: true,
    error: {
      code: 400,
      description:
        "prompt has 9895 tokens but each agent sequence holds 8192 tokens",
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
