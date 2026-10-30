import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema joins tool call validation errors into a non-terminal unprocessable error", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      ToolCallValidationFailed: [
        "missing field 'location'",
        "extra field 'foo'",
      ],
    }),
  );

  strictEqual(parsed.done, false);
  deepStrictEqual(parsed.error, {
    code: 422,
    description: "missing field 'location'; extra field 'foo'",
  });
});
