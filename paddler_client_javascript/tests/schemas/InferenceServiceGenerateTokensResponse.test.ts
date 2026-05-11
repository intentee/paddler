import test from "ava";

import { InferenceServiceGenerateTokensResponseSchema } from "../../src/schemas/InferenceServiceGenerateTokensResponse";

test("ContentToken normalises into a streaming token with content kind", function (t) {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-1",
      response: { GeneratedToken: { ContentToken: "Hello" } },
    },
  });

  t.is(parsed.done, false);
  t.is(parsed.error, null);
  t.is(parsed.token, "Hello");
  t.is(parsed.tokenKind, "content");
  t.is(parsed.toolCalls, null);
});

test("ReasoningToken maps to reasoning kind", function (t) {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-2",
      response: { GeneratedToken: { ReasoningToken: "thinking..." } },
    },
  });

  t.is(parsed.token, "thinking...");
  t.is(parsed.tokenKind, "reasoning");
});

test("Done normalises with the full usage summary", function (t) {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-3",
      response: {
        GeneratedToken: {
          Done: {
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

  t.is(parsed.done, true);
  t.is(parsed.error, null);
  t.deepEqual(parsed.summary?.usage.prompt_tokens, 10);
});

test("ToolCallValidatorBuildFailed normalises to a terminal error", function (t) {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-4",
      response: {
        GeneratedToken: {
          ToolCallValidatorBuildFailed: "schema invalid",
        },
      },
    },
  });

  t.is(parsed.done, true);
  t.deepEqual(parsed.error, { code: 400, description: "schema invalid" });
});

test("Top-level Error envelope normalises to terminal error", function (t) {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Error: {
      request_id: "req-5",
      error: { code: 500, description: "boom" },
    },
  });

  t.is(parsed.done, true);
  t.deepEqual(parsed.error, { code: 500, description: "boom" });
});

test("UnrecognizedToolCallFormat preserves text and FFI error message", function (t) {
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

  t.is(parsed.done, false);
  t.is(parsed.error, null);
  t.is(parsed.ok, true);
  t.is(parsed.token, null);
  t.is(parsed.tokenKind, null);
  t.is(parsed.toolCalls, null);
  t.deepEqual(parsed.rawToolCallTokens, {
    text: "<unknown>raw</unknown>",
    ffi_error_message: "common_chat_parse failed: no parser",
  });
});

test("ImageExceedsBatchSize is terminal and describes token counts", function (t) {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-7",
      response: {
        GeneratedToken: {
          ImageExceedsBatchSize: { image_tokens: 368, n_batch: 100 },
        },
      },
    },
  });

  t.is(parsed.done, true);
  t.is(parsed.ok, false);
  t.not(parsed.error, null);
  t.is(parsed.error?.code, 400);
  t.true(parsed.error?.description.includes("368"));
  t.true(parsed.error?.description.includes("100"));
});
