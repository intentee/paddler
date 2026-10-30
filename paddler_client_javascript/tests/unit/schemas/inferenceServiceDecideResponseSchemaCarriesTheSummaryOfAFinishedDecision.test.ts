import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";
import { decisionResponse } from "./decisionResponse";

test("InferenceServiceDecideResponseSchema carries the summary of a finished decision", function () {
  deepStrictEqual(
    InferenceServiceDecideResponseSchema.parse(
      decisionResponse({
        Done: { input_tokens: 85, processing_milliseconds: 12 },
      }),
    ).summary,
    { input_tokens: 85, processing_milliseconds: 12 },
  );
});
