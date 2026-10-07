import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";

function generatedToken(generatedTokenResult: unknown): unknown {
  return {
    Response: {
      generated_by: null,
      request_id: "req-1",
      response: { GeneratedToken: generatedTokenResult },
    },
  };
}

const terminalErrorCodes = [
  { code: 500, variant: "BatchAssemblyFailed" },
  { code: 500, variant: "ChatTemplateError" },
  { code: 500, variant: "DecodeFailed" },
  { code: 500, variant: "DetokenizationFailed" },
  { code: 400, variant: "GrammarIncompatibleWithThinking" },
  { code: 500, variant: "GrammarInitializationFailed" },
  { code: 500, variant: "GrammarRejectedModelOutput" },
  { code: 400, variant: "GrammarSyntaxError" },
  { code: 400, variant: "ImageDecodingFailed" },
  { code: 500, variant: "KvCacheClearFailed" },
  { code: 500, variant: "MediaMicroBatchCheckFailed" },
  { code: 503, variant: "ModelNotLoaded" },
  { code: 500, variant: "MultimodalIngestionFailed" },
  { code: 400, variant: "MultimodalNotSupported" },
  { code: 400, variant: "MultimodalTokenizationFailed" },
  { code: 503, variant: "NoSequenceSlotAvailable" },
  { code: 400, variant: "PromptTokenizationFailed" },
  { code: 500, variant: "SamplerChainCreationFailed" },
  { code: 500, variant: "SamplerError" },
  { code: 500, variant: "SamplingCandidatesExhausted" },
  { code: 503, variant: "SchedulerUnavailable" },
  { code: 500, variant: "SequenceIdOutOfRange" },
  { code: 503, variant: "InferenceModeMismatch" },
  { code: 400, variant: "ToolSchemaInvalid" },
  { code: 500, variant: "ToolsSerializationFailed" },
] as const;

for (const { code, variant } of terminalErrorCodes) {
  test(`${variant} normalises to a terminal error with code ${code}`, function () {
    const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
      generatedToken({ [variant]: "described failure" }),
    );

    strictEqual(parsed.done, true);
    deepStrictEqual(parsed.error, { code, description: "described failure" });
  });
}

const streamingTokenKinds = [
  { tokenKind: "tool_call", variant: "ToolCallToken" },
  { tokenKind: "undeterminable", variant: "UndeterminableToken" },
] as const;

for (const { tokenKind, variant } of streamingTokenKinds) {
  test(`${variant} normalises into a streaming token with ${tokenKind} kind`, function () {
    const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
      generatedToken({ [variant]: "piece" }),
    );

    strictEqual(parsed.done, false);
    strictEqual(parsed.token, "piece");
    strictEqual(parsed.tokenKind, tokenKind);
  });
}

test("ToolCallParsed carries the parsed calls without ending the stream", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      ToolCallParsed: [
        {
          arguments: { ValidJson: { location: "Paris" } },
          id: "call_42",
          name: "get_weather",
        },
      ],
    }),
  );

  strictEqual(parsed.done, false);
  deepStrictEqual(parsed.toolCalls, [
    {
      arguments: { ValidJson: { location: "Paris" } },
      id: "call_42",
      name: "get_weather",
    },
  ]);
});

test("ToolCallParseFailed is a non-terminal unprocessable error", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({ ToolCallParseFailed: "syntax error at 12" }),
  );

  strictEqual(parsed.done, false);
  deepStrictEqual(parsed.error, {
    code: 422,
    description: "syntax error at 12",
  });
});

test("ToolCallValidationFailed joins its errors into a non-terminal unprocessable error", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
    generatedToken({
      ToolCallValidationFailed: [
        "missing field 'location'",
        "extra field 'foo'",
      ],
    }),
  );

  strictEqual(parsed.done, false);
  deepStrictEqual(parsed.error, {
    code: 422,
    description: "missing field 'location'; extra field 'foo'",
  });
});

test("ContentToken normalises into a streaming token with content kind", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-1",
      response: { GeneratedToken: { ContentToken: "Hello" } },
    },
  });

  strictEqual(parsed.done, false);
  strictEqual(parsed.error, null);
  strictEqual(parsed.token, "Hello");
  strictEqual(parsed.tokenKind, "content");
  strictEqual(parsed.toolCalls, null);
});

test("ReasoningToken maps to reasoning kind", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-2",
      response: { GeneratedToken: { ReasoningToken: "thinking..." } },
    },
  });

  strictEqual(parsed.token, "thinking...");
  strictEqual(parsed.tokenKind, "reasoning");
});

test("Done normalises with the end reason and the full usage summary", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-3",
      response: {
        GeneratedToken: {
          Done: {
            finish: "MaxTokens",
            usage: {
              prompt_tokens: 10,
              cached_prompt_tokens: 0,
              input_image_tokens: 0,
              input_audio_tokens: 0,
              content_tokens: 5,
              reasoning_tokens: 0,
              tool_call_tokens: 0,
              undeterminable_tokens: 0,
            },
          },
        },
      },
    },
  });

  strictEqual(parsed.done, true);
  strictEqual(parsed.error, null);
  strictEqual(parsed.summary?.finish, "MaxTokens");
  deepStrictEqual(parsed.summary?.usage.prompt_tokens, 10);
});

test("Top-level Error envelope normalises to terminal error", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Error: {
      request_id: "req-5",
      error: { code: 500, description: "boom" },
    },
  });

  strictEqual(parsed.done, true);
  deepStrictEqual(parsed.error, { code: 500, description: "boom" });
});

test("UnrecognizedToolCallFormat preserves text and FFI error message", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-6",
      response: {
        GeneratedToken: {
          UnrecognizedToolCallFormat: {
            text: "<unknown>raw</unknown>",
            ffi_error_message: "common_chat_parse failed: no parser",
          },
        },
      },
    },
  });

  strictEqual(parsed.done, false);
  strictEqual(parsed.error, null);
  strictEqual(parsed.ok, true);
  strictEqual(parsed.token, null);
  strictEqual(parsed.tokenKind, null);
  strictEqual(parsed.toolCalls, null);
  deepStrictEqual(parsed.rawToolCallTokens, {
    text: "<unknown>raw</unknown>",
    ffi_error_message: "common_chat_parse failed: no parser",
  });
});

test("PromptExceedsContextSize is terminal and describes token counts", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-8",
      response: {
        GeneratedToken: {
          PromptExceedsContextSize: {
            prompt_tokens: 9895,
            sequence_context_size: 8192,
          },
        },
      },
    },
  });

  deepStrictEqual(parsed, {
    done: true,
    error: {
      code: 400,
      description:
        "prompt has 9895 tokens but each agent sequence holds 8192 tokens",
    },
    generated_by: null,
    ok: false,
    rawToolCallTokens: null,
    request_id: "req-8",
    summary: null,
    token: null,
    tokenKind: null,
    toolCalls: null,
  });
});

test("MediaExceedsMicroBatch is terminal and describes token counts", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-7",
      response: {
        GeneratedToken: {
          MediaExceedsMicroBatch: {
            media_tokens: 368,
            micro_batch_tokens: 100,
          },
        },
      },
    },
  });

  deepStrictEqual(parsed, {
    done: true,
    error: {
      code: 400,
      description:
        "media required 368 tokens but one agent micro batch holds 100 tokens",
    },
    generated_by: null,
    ok: false,
    rawToolCallTokens: null,
    request_id: "req-7",
    summary: null,
    token: null,
    tokenKind: null,
    toolCalls: null,
  });
});
