import { z } from "zod";

import { ParsedToolCallSchema } from "./ParsedToolCall";

export type GeneratedTokenKind =
  | "content"
  | "reasoning"
  | "tool_call"
  | "undeterminable";

const TokenUsageSchema = z.object({
  prompt_tokens: z.number(),
  cached_prompt_tokens: z.number(),
  input_image_tokens: z.number(),
  input_audio_tokens: z.number(),
  content_tokens: z.number(),
  reasoning_tokens: z.number(),
  tool_call_tokens: z.number(),
  undeterminable_tokens: z.number(),
});

const GenerationSummarySchema = z.object({
  usage: TokenUsageSchema,
});

const RawToolCallTokensSchema = z.object({
  text: z.string(),
  ffi_error_message: z.string(),
});

const OversizedImageDetailsSchema = z.object({
  image_tokens: z.number(),
  n_batch: z.number(),
});

const GeneratedTokenResultSchema = z.union([
  z.object({ ContentToken: z.string() }),
  z.object({ ReasoningToken: z.string() }),
  z.object({ ToolCallToken: z.string() }),
  z.object({ UndeterminableToken: z.string() }),
  z.object({ Done: GenerationSummarySchema }),
  z.object({ ChatTemplateError: z.string() }),
  z.object({ GrammarIncompatibleWithThinking: z.string() }),
  z.object({ GrammarInitializationFailed: z.string() }),
  z.object({ GrammarRejectedModelOutput: z.string() }),
  z.object({ GrammarSyntaxError: z.string() }),
  z.object({ ImageDecodingFailed: z.string() }),
  z.object({ ImageExceedsBatchSize: OversizedImageDetailsSchema }),
  z.object({ MultimodalNotSupported: z.string() }),
  z.object({ SamplerError: z.string() }),
  z.object({ ToolCallParsed: z.array(ParsedToolCallSchema) }),
  z.object({ ToolCallParseFailed: z.string() }),
  z.object({ ToolCallValidationFailed: z.array(z.string()) }),
  z.object({ ToolCallValidatorBuildFailed: z.string() }),
  z.object({ UnrecognizedToolCallFormat: RawToolCallTokensSchema }),
]);

type Normalised =
  | {
      done: true;
      error: null;
      generated_by: string | null;
      ok: true;
      rawToolCallTokens: null;
      request_id: string;
      summary: z.infer<typeof GenerationSummarySchema>;
      token: null;
      tokenKind: null;
      toolCalls: null;
    }
  | {
      done: false;
      error: null;
      generated_by: string | null;
      ok: true;
      rawToolCallTokens: null;
      request_id: string;
      summary: null;
      token: string;
      tokenKind: GeneratedTokenKind;
      toolCalls: null;
    }
  | {
      done: false;
      error: null;
      generated_by: string | null;
      ok: true;
      rawToolCallTokens: null;
      request_id: string;
      summary: null;
      token: null;
      tokenKind: null;
      toolCalls: ReadonlyArray<z.infer<typeof ParsedToolCallSchema>>;
    }
  | {
      done: false;
      error: null;
      generated_by: string | null;
      ok: true;
      rawToolCallTokens: z.infer<typeof RawToolCallTokensSchema>;
      request_id: string;
      summary: null;
      token: null;
      tokenKind: null;
      toolCalls: null;
    }
  | {
      done: true;
      error: { code: number; description: string };
      generated_by: string | null;
      ok: false;
      rawToolCallTokens: null;
      request_id: string;
      summary: null;
      token: null;
      tokenKind: null;
      toolCalls: null;
    }
  | {
      done: false;
      error: { code: number; description: string };
      generated_by: string | null;
      ok: false;
      rawToolCallTokens: null;
      request_id: string;
      summary: null;
      token: null;
      tokenKind: null;
      toolCalls: null;
    };

function terminalError(
  request_id: string,
  generated_by: string | null,
  code: number,
  description: string,
): Normalised {
  return Object.freeze({
    done: true,
    error: Object.freeze({ code, description }),
    generated_by,
    ok: false,
    rawToolCallTokens: null,
    request_id,
    summary: null,
    token: null,
    tokenKind: null,
    toolCalls: null,
  });
}

function nonTerminalError(
  request_id: string,
  generated_by: string | null,
  code: number,
  description: string,
): Normalised {
  return Object.freeze({
    done: false,
    error: Object.freeze({ code, description }),
    generated_by,
    ok: false,
    rawToolCallTokens: null,
    request_id,
    summary: null,
    token: null,
    tokenKind: null,
    toolCalls: null,
  });
}

function streamingToken(
  request_id: string,
  generated_by: string | null,
  token: string,
  tokenKind: GeneratedTokenKind,
): Normalised {
  return Object.freeze({
    done: false,
    error: null,
    generated_by,
    ok: true,
    rawToolCallTokens: null,
    request_id,
    summary: null,
    token,
    tokenKind,
    toolCalls: null,
  });
}

function unrecognizedToolCallFormat(
  request_id: string,
  generated_by: string | null,
  raw: z.infer<typeof RawToolCallTokensSchema>,
): Normalised {
  return Object.freeze({
    done: false,
    error: null,
    generated_by,
    ok: true,
    rawToolCallTokens: Object.freeze(raw),
    request_id,
    summary: null,
    token: null,
    tokenKind: null,
    toolCalls: null,
  });
}

export const InferenceServiceGenerateTokensResponseSchema = z
  .union([
    z.object({
      Error: z.object({
        error: z.object({
          code: z.number(),
          description: z.string(),
        }),
        request_id: z.string(),
      }),
    }),
    z.object({
      Response: z.object({
        generated_by: z.string().nullable(),
        request_id: z.string(),
        response: z.object({
          GeneratedToken: GeneratedTokenResultSchema,
        }),
      }),
    }),
  ])
  .transform(function (data): Normalised {
    if ("Error" in data) {
      return terminalError(
        data.Error.request_id,
        null,
        data.Error.error.code,
        data.Error.error.description,
      );
    }

    const request_id = data.Response.request_id;
    const generated_by = data.Response.generated_by;
    const variant = data.Response.response.GeneratedToken;

    if ("ContentToken" in variant) {
      return streamingToken(request_id, generated_by, variant.ContentToken, "content");
    }

    if ("ReasoningToken" in variant) {
      return streamingToken(request_id, generated_by, variant.ReasoningToken, "reasoning");
    }

    if ("ToolCallToken" in variant) {
      return streamingToken(request_id, generated_by, variant.ToolCallToken, "tool_call");
    }

    if ("UndeterminableToken" in variant) {
      return streamingToken(
        request_id,
        generated_by,
        variant.UndeterminableToken,
        "undeterminable",
      );
    }

    if ("Done" in variant) {
      return Object.freeze({
        done: true,
        error: null,
        generated_by,
        ok: true,
        rawToolCallTokens: null,
        request_id,
        summary: variant.Done,
        token: null,
        tokenKind: null,
        toolCalls: null,
      });
    }

    if ("ToolCallParsed" in variant) {
      return Object.freeze({
        done: false,
        error: null,
        generated_by,
        ok: true,
        rawToolCallTokens: null,
        request_id,
        summary: null,
        token: null,
        tokenKind: null,
        toolCalls: Object.freeze(variant.ToolCallParsed),
      });
    }

    if ("UnrecognizedToolCallFormat" in variant) {
      return unrecognizedToolCallFormat(
        request_id,
        generated_by,
        variant.UnrecognizedToolCallFormat,
      );
    }

    if ("ToolCallParseFailed" in variant) {
      return nonTerminalError(request_id, generated_by, 422, variant.ToolCallParseFailed);
    }

    if ("ToolCallValidationFailed" in variant) {
      return nonTerminalError(
        request_id,
        generated_by,
        422,
        variant.ToolCallValidationFailed.join("; "),
      );
    }

    if ("ToolCallValidatorBuildFailed" in variant) {
      return terminalError(
        request_id,
        generated_by,
        400,
        variant.ToolCallValidatorBuildFailed,
      );
    }

    if ("ChatTemplateError" in variant) {
      return terminalError(request_id, generated_by, 500, variant.ChatTemplateError);
    }

    if ("GrammarIncompatibleWithThinking" in variant) {
      return terminalError(
        request_id,
        generated_by,
        400,
        variant.GrammarIncompatibleWithThinking,
      );
    }

    if ("GrammarInitializationFailed" in variant) {
      return terminalError(request_id, generated_by, 500, variant.GrammarInitializationFailed);
    }

    if ("GrammarRejectedModelOutput" in variant) {
      return terminalError(request_id, generated_by, 500, variant.GrammarRejectedModelOutput);
    }

    if ("GrammarSyntaxError" in variant) {
      return terminalError(request_id, generated_by, 400, variant.GrammarSyntaxError);
    }

    if ("ImageDecodingFailed" in variant) {
      return terminalError(request_id, generated_by, 400, variant.ImageDecodingFailed);
    }

    if ("ImageExceedsBatchSize" in variant) {
      const details = variant.ImageExceedsBatchSize;
      return terminalError(
        request_id,
        generated_by,
        400,
        `image required ${details.image_tokens} tokens but n_batch is ${details.n_batch}`,
      );
    }

    if ("MultimodalNotSupported" in variant) {
      return terminalError(request_id, generated_by, 400, variant.MultimodalNotSupported);
    }

    return terminalError(request_id, generated_by, 500, variant.SamplerError);
  });

export type InferenceServiceGenerateTokensResponse = z.infer<
  typeof InferenceServiceGenerateTokensResponseSchema
>;
