import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

test("InferenceServiceGenerateTokensResponseSchema preserves the text and expected format of an unrecognized tool call format", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      UnrecognizedToolCallFormat: {
        text: "<unknown>raw</unknown>",
        expected_format: "peg-native",
      },
    }),
  );

  strictEqual(parsed.done, false);
  strictEqual(parsed.error, null);
  strictEqual(parsed.ok, true);
  strictEqual(parsed.token, null);
  strictEqual(parsed.tokenKind, null);
  strictEqual(parsed.toolCalls, null);
  deepStrictEqual(parsed.rawToolCallTokens, {
    text: "<unknown>raw</unknown>",
    expected_format: "peg-native",
  });
});
