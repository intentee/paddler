import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema ends the stream with the finish reason and the usage summary", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      Done: {
        finish: "MaxTokens",
        usage: {
          prompt_tokens: 10,
          cached_prompt_tokens: 0,
          input_image_tokens: 0,
          input_audio_tokens: 0,
          content_tokens: 5,
          reasoning_tokens: 0,
          tool_call_tokens: 0,
          undeterminable_tokens: 0,
        },
      },
    }),
  );

  strictEqual(parsed.done, true);
  strictEqual(parsed.error, null);
  strictEqual(parsed.summary?.finish, "MaxTokens");
  deepStrictEqual(parsed.summary?.usage.prompt_tokens, 10);
});
