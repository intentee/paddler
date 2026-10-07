import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";

function decisionResponse(decisionResult: unknown): unknown {
  return {
    Response: {
      generated_by: "agent-1",
      request_id: "req-decision",
      response: { Decision: decisionResult },
    },
  };
}

const decisionFailureCodes = [
  { code: 500, variant: "AgentRuntimeFailed" },
  { code: 500, variant: "BatchAssemblyFailed" },
  { code: 500, variant: "DecodeFailed" },
  { code: 500, variant: "HiddenStateUnavailable" },
  { code: 503, variant: "InferenceModeMismatch" },
  { code: 400, variant: "InputTokenizationFailed" },
  { code: 500, variant: "KvCacheCopyFailed" },
  { code: 500, variant: "KvCacheRemovalFailed" },
  { code: 503, variant: "ModelNotLoaded" },
  { code: 503, variant: "SchedulerUnavailable" },
];

test("an answered question keeps the decision stream open", function () {
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

test("a finished decision carries its summary", function () {
  deepStrictEqual(
    InferenceServiceDecideResponseSchema.parse(
      decisionResponse({
        Done: { input_tokens: 85, processing_milliseconds: 12 },
      }),
    ).summary,
    { input_tokens: 85, processing_milliseconds: 12 },
  );
});

test("a decision exceeding the context fails as a bad request", function () {
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

test("every decision failure ends the stream with its status code", function () {
  for (const { code, variant } of decisionFailureCodes) {
    const message = InferenceServiceDecideResponseSchema.parse(
      decisionResponse({ [variant]: `agent-1: ${variant}` }),
    );

    strictEqual(message.done, true, variant);
    deepStrictEqual(
      message.error,
      { code, description: `agent-1: ${variant}` },
      variant,
    );
  }
});

test("a wire error ends the decision stream", function () {
  deepStrictEqual(
    InferenceServiceDecideResponseSchema.parse({
      Error: {
        error: { code: 504, description: "timed out" },
        request_id: "req-decision",
      },
    }).error,
    { code: 504, description: "timed out" },
  );
});

test("a decision result with more than one failure is refused", function () {
  strictEqual(
    InferenceServiceDecideResponseSchema.safeParse(
      decisionResponse({ DecodeFailed: "first", ModelNotLoaded: "second" }),
    ).success,
    false,
  );
});
