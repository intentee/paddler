import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema reports a tool call parse failure as a non-terminal unprocessable error", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({ ToolCallParseFailed: "syntax error at 12" }),
  );

  strictEqual(parsed.done, false);
  deepStrictEqual(parsed.error, {
    code: 422,
    description: "syntax error at 12",
  });
});
