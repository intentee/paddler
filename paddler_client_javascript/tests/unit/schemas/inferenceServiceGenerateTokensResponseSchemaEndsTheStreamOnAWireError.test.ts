import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";

test("InferenceServiceGenerateTokensResponseSchema ends the stream on a wire error", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Error: {
      request_id: "req-5",
      error: { code: 500, description: "boom" },
    },
  });

  strictEqual(parsed.done, true);
  deepStrictEqual(parsed.error, { code: 500, description: "boom" });
});
