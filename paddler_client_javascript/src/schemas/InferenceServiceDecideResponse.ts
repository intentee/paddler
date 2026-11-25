import { z } from "zod";

const DecisionAnswerSchema = z.discriminatedUnion("type", [
  z
    .object({
      choice: z.string(),
      confidence: z.number(),
      probabilities: z.record(z.string(), z.number()),
      type: z.literal("choice"),
    })
    .strict(),
  z
    .object({
      noul: z.number(),
      type: z.literal("noul"),
    })
    .strict(),
  z
    .object({
      confidence: z.number(),
      legend: z.record(z.string(), z.json()),
      probabilities: z.record(z.string(), z.number()),
      score: z.number(),
      type: z.literal("score"),
    })
    .strict(),
]);

const AnsweredDecisionQuestionSchema = z
  .object({
    answer: DecisionAnswerSchema,
    id: z.string(),
  })
  .strict();

const DecisionSummarySchema = z.object({
  input_tokens: z.number(),
  processing_milliseconds: z.number(),
});

const OversizedDecisionDetailsSchema = z.object({
  context_size: z.number(),
  required_tokens: z.number(),
});

const decisionFailureCodes = {
  AnswersUnreadable: 500,
  BatchAssemblyFailed: 500,
  DecisionsNotServed: 503,
  DecodeFailed: 500,
  KvCacheCopyFailed: 500,
  KvCacheRemovalFailed: 500,
  MediaEvaluationFailed: 500,
  ModelNotLoaded: 503,
  RequestRejected: 400,
  SchedulerUnavailable: 503,
} as const;

type DecisionFailure = keyof typeof decisionFailureCodes;

const DecisionFailureSchema = z.union([
  z.object({ AnswersUnreadable: z.string() }).strict(),
  z.object({ BatchAssemblyFailed: z.string() }).strict(),
  z.object({ DecisionsNotServed: z.string() }).strict(),
  z.object({ DecodeFailed: z.string() }).strict(),
  z.object({ KvCacheCopyFailed: z.string() }).strict(),
  z.object({ KvCacheRemovalFailed: z.string() }).strict(),
  z.object({ MediaEvaluationFailed: z.string() }).strict(),
  z.object({ ModelNotLoaded: z.string() }).strict(),
  z.object({ RequestRejected: z.string() }).strict(),
  z.object({ SchedulerUnavailable: z.string() }).strict(),
]);

const DecisionResultSchema = z.union([
  z.object({ Done: DecisionSummarySchema }).strict(),
  z.object({ QuestionAnswered: AnsweredDecisionQuestionSchema }).strict(),
  z.object({ RequestExceedsContext: OversizedDecisionDetailsSchema }).strict(),
  z.literal("StopRequested"),
  DecisionFailureSchema,
]);

type DecisionMessage =
  | {
      answered_question: z.infer<typeof AnsweredDecisionQuestionSchema>;
      done: false;
      error: null;
      generated_by: string | null;
      request_id: string;
      summary: null;
    }
  | {
      answered_question: null;
      done: true;
      error: null;
      generated_by: string | null;
      request_id: string;
      summary: z.infer<typeof DecisionSummarySchema>;
    }
  | {
      answered_question: null;
      done: true;
      error: { code: number; description: string };
      generated_by: string | null;
      request_id: string;
      summary: null;
    };

function failedDecision(
  request_id: string,
  generated_by: string | null,
  code: number,
  description: string,
): DecisionMessage {
  return Object.freeze({
    answered_question: null,
    done: true,
    error: Object.freeze({ code, description }),
    generated_by,
    request_id,
    summary: null,
  });
}

const DecisionStreamMessageSchema = z.union([
  z.object({
    Error: z.object({
      error: z.object({ code: z.number(), description: z.string() }),
      request_id: z.string(),
    }),
  }),
  z.object({
    Response: z.object({
      generated_by: z.string().nullable(),
      request_id: z.string(),
      response: z.object({ Decision: DecisionResultSchema }),
    }),
  }),
]);

function decisionMessage(
  message: z.infer<typeof DecisionStreamMessageSchema>,
): DecisionMessage {
  if ("Error" in message) {
    return failedDecision(
      message.Error.request_id,
      null,
      message.Error.error.code,
      message.Error.error.description,
    );
  }

  const { generated_by, request_id, response } = message.Response;
  const decisionResult = response.Decision;

  if (decisionResult === "StopRequested") {
    return failedDecision(
      request_id,
      generated_by,
      502,
      "the agent stopped before it finished the decision",
    );
  }

  if ("Done" in decisionResult) {
    return Object.freeze({
      answered_question: null,
      done: true,
      error: null,
      generated_by,
      request_id,
      summary: decisionResult.Done,
    });
  }

  if ("QuestionAnswered" in decisionResult) {
    return Object.freeze({
      answered_question: decisionResult.QuestionAnswered,
      done: false,
      error: null,
      generated_by,
      request_id,
      summary: null,
    });
  }

  if ("RequestExceedsContext" in decisionResult) {
    const { context_size, required_tokens } =
      decisionResult.RequestExceedsContext;

    return failedDecision(
      request_id,
      generated_by,
      400,
      `the decision needs ${required_tokens} tokens in the cache, more than the ${context_size} the context holds`,
    );
  }

  const [[failure, description]] = Object.entries(decisionResult) as [
    [DecisionFailure, string],
  ];

  return failedDecision(
    request_id,
    generated_by,
    decisionFailureCodes[failure],
    description,
  );
}

export const InferenceServiceDecideResponseSchema =
  DecisionStreamMessageSchema.transform(decisionMessage);

export type InferenceServiceDecideResponse = z.infer<
  typeof InferenceServiceDecideResponseSchema
>;
