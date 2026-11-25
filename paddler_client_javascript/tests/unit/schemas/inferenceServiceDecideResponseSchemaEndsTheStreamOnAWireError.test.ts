import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";

test("InferenceServiceDecideResponseSchema ends the stream on a wire error", function () {
  const parsed = InferenceServiceDecideResponseSchema.parse({
    Error: {
      error: { code: 504, description: "timed out" },
      request_id: "req-decision",
    },
  });

  strictEqual(parsed.done, true);
  deepStrictEqual(parsed.error, { code: 504, description: "timed out" });
});
