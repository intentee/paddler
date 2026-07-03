import {
  deepStrictEqual,
  notStrictEqual,
  ok,
  strictEqual,
} from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../src/schemas/InferenceServiceGenerateTokensResponse";

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

test("Done normalises with the full usage summary", function () {
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

  strictEqual(parsed.done, true);
  strictEqual(parsed.error, null);
  deepStrictEqual(parsed.summary?.usage.prompt_tokens, 10);
});

test("ToolCallValidatorBuildFailed normalises to a terminal error", function () {
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

  strictEqual(parsed.done, true);
  deepStrictEqual(parsed.error, { code: 400, description: "schema invalid" });
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

test("TokenGenerationDisabled normalises to a terminal error", function () {
  const parsed = InferenceServiceGenerateTokensResponseSchema.parse({
    Response: {
      generated_by: null,
      request_id: "req-8",
      response: {
        GeneratedToken: {
          TokenGenerationDisabled: "cluster is configured for embeddings",
        },
      },
    },
  });

  strictEqual(parsed.done, true);
  deepStrictEqual(parsed.error, {
    code: 501,
    description: "cluster is configured for embeddings",
  });
});

test("ImageExceedsBatchSize is terminal and describes token counts", function () {
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

  strictEqual(parsed.done, true);
  strictEqual(parsed.ok, false);
  notStrictEqual(parsed.error, null);
  strictEqual(parsed.error?.code, 400);
  ok(parsed.error?.description.includes("368"));
  ok(parsed.error?.description.includes("100"));
});
