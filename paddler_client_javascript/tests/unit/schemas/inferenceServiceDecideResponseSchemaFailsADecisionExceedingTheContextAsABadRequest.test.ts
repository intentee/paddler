import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";
import { decisionResponse } from "./decisionResponse";

test("InferenceServiceDecideResponseSchema fails a decision exceeding the context as a bad request", function () {
  deepStrictEqual(
    InferenceServiceDecideResponseSchema.parse(
      decisionResponse({
        RequestExceedsContext: { context_size: 256, required_tokens: 900 },
      }),
    ).error,
    {
      code: 400,
      description:
        "the decision needs 900 tokens in the cache, more than the 256 the context holds",
    },
  );
});
