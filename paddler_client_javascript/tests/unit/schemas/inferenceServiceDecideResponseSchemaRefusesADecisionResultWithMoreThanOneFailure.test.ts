import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";
import { decisionResponse } from "./decisionResponse";

test("InferenceServiceDecideResponseSchema refuses a decision result with more than one failure", function () {
  strictEqual(
    InferenceServiceDecideResponseSchema.safeParse(
      decisionResponse({ DecodeFailed: "first", ModelNotLoaded: "second" }),
    ).success,
    false,
  );
});
