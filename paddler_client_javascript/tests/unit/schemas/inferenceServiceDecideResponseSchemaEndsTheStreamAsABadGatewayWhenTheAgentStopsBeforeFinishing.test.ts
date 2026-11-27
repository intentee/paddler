import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";
import { decisionResponse } from "./decisionResponse";

test("InferenceServiceDecideResponseSchema ends the stream as a bad gateway when the agent stops before finishing", function () {
  deepStrictEqual(
    InferenceServiceDecideResponseSchema.parse(
      decisionResponse("StopRequested"),
    ),
    {
      answered_question: null,
      done: true,
      error: {
        code: 502,
        description: "the agent stopped before it finished the decision",
      },
      generated_by: "agent-1",
      request_id: "req-decision",
      summary: null,
    },
  );
});
