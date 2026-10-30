import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";
import { decisionResponse } from "./decisionResponse";

test("InferenceServiceDecideResponseSchema keeps the stream open after an answered question", function () {
  deepStrictEqual(
    InferenceServiceDecideResponseSchema.parse(
      decisionResponse({
        QuestionAnswered: { id: "paid", probabilities: [0.25, 0.75] },
      }),
    ),
    {
      answer: { id: "paid", probabilities: [0.25, 0.75] },
      done: false,
      error: null,
      generated_by: "agent-1",
      request_id: "req-decision",
      summary: null,
    },
  );
});
