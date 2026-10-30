import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

const streamingTokenKinds = [
  { token: "Hello", tokenKind: "content", variant: "ContentToken" },
  { token: "thinking...", tokenKind: "reasoning", variant: "ReasoningToken" },
  { token: "piece", tokenKind: "tool_call", variant: "ToolCallToken" },
  {
    token: "piece",
    tokenKind: "undeterminable",
    variant: "UndeterminableToken",
  },
] as const;

test("InferenceServiceGenerateTokensResponseSchema streams every token variant with its kind", function () {
  for (const { token, tokenKind, variant } of streamingTokenKinds) {
    const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
      generatedToken({ [variant]: token }),
    );

    strictEqual(parsed.done, false, variant);
    strictEqual(parsed.error, null, variant);
    strictEqual(parsed.token, token, variant);
    strictEqual(parsed.tokenKind, tokenKind, variant);
    strictEqual(parsed.toolCalls, null, variant);
  }
});
