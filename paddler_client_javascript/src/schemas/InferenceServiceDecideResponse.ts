import { z } from "zod";

const DecisionAnswerSchema = z.object({
  id: z.string(),
  probabilities: z.array(z.number()),
});

const DecisionSummarySchema = z.object({
  input_tokens: z.number(),
  processing_milliseconds: z.number(),
});

const OversizedDecisionDetailsSchema = z.object({
  context_size: z.number(),
  required_tokens: z.number(),
});

const decisionFailureCodes = {
  AgentRuntimeFailed: 500,
  BatchAssemblyFailed: 500,
  DecodeFailed: 500,
  HiddenStateUnavailable: 500,
  InferenceModeMismatch: 503,
  InputTokenizationFailed: 400,
  KvCacheCopyFailed: 500,
  KvCacheRemovalFailed: 500,
  ModelNotLoaded: 503,
  SchedulerUnavailable: 503,
} as const;

type DecisionFailure = keyof typeof decisionFailureCodes;

const DecisionFailureSchema = z.union([
  z.object({ AgentRuntimeFailed: z.string() }).strict(),
  z.object({ BatchAssemblyFailed: z.string() }).strict(),
  z.object({ DecodeFailed: z.string() }).strict(),
  z.object({ HiddenStateUnavailable: z.string() }).strict(),
  z.object({ InferenceModeMismatch: z.string() }).strict(),
  z.object({ InputTokenizationFailed: z.string() }).strict(),
  z.object({ KvCacheCopyFailed: z.string() }).strict(),
  z.object({ KvCacheRemovalFailed: z.string() }).strict(),
  z.object({ ModelNotLoaded: z.string() }).strict(),
  z.object({ SchedulerUnavailable: z.string() }).strict(),
]);

const DecisionResultSchema = z.union([
  z.object({ Done: DecisionSummarySchema }).strict(),
  z.object({ QuestionAnswered: DecisionAnswerSchema }).strict(),
  z.object({ RequestExceedsContext: OversizedDecisionDetailsSchema }).strict(),
  DecisionFailureSchema,
]);

type DecisionMessage =
  | {
      answer: z.infer<typeof DecisionAnswerSchema>;
      done: false;
      error: null;
      generated_by: string | null;
      request_id: string;
      summary: null;
    }
  | {
      answer: null;
      done: true;
      error: null;
      generated_by: string | null;
      request_id: string;
      summary: z.infer<typeof DecisionSummarySchema>;
    }
  | {
      answer: null;
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
    answer: null,
    done: true,
    error: Object.freeze({ code, description }),
    generated_by,
    request_id,
    summary: null,
  });
}

export const InferenceServiceDecideResponseSchema = z
  .union([
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
  ])
  .transform(function (message): DecisionMessage {
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

    if ("Done" in decisionResult) {
      return Object.freeze({
        answer: null,
        done: true,
        error: null,
        generated_by,
        request_id,
        summary: decisionResult.Done,
      });
    }

    if ("QuestionAnswered" in decisionResult) {
      return Object.freeze({
        answer: decisionResult.QuestionAnswered,
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
  });

export type InferenceServiceDecideResponse = z.infer<
  typeof InferenceServiceDecideResponseSchema
>;
