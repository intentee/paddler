import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";
import { decisionResponse } from "./decisionResponse";

const answeredQuestion = {
  answer: {
    choice: "yes",
    confidence: 0.75,
    probabilities: { no: 0.25, yes: 0.75 },
    type: "choice",
  },
  id: "paid",
};

test("InferenceServiceDecideResponseSchema keeps the stream open after an answered question", function () {
  deepStrictEqual(
    InferenceServiceDecideResponseSchema.parse(
      decisionResponse({ QuestionAnswered: answeredQuestion }),
    ),
    {
      answered_question: answeredQuestion,
      done: false,
      error: null,
      generated_by: "agent-1",
      request_id: "req-decision",
      summary: null,
    },
  );
});
