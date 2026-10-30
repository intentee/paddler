import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceGenerateTokensResponseSchema } from "../../../src/schemas/InferenceServiceGenerateTokensResponse";
import { generatedToken } from "./generatedToken";

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
  { code: 400, variant: "PromptTokenizedToNothing" },
  { code: 500, variant: "SamplerChainCreationFailed" },
  { code: 500, variant: "SamplerError" },
  { code: 500, variant: "SamplingCandidatesExhausted" },
  { code: 503, variant: "SchedulerUnavailable" },
  { code: 400, variant: "ToolSchemaInvalid" },
] as const;

test("InferenceServiceGenerateTokensResponseSchema ends the stream with the status code of every terminal failure", function () {
  for (const { code, variant } of terminalErrorCodes) {
    const parsed = InferenceServiceGenerateTokensResponseSchema.parse(
      generatedToken({ [variant]: "described failure" }),
    );

    strictEqual(parsed.done, true, variant);
    deepStrictEqual(
      parsed.error,
      { code, description: "described failure" },
      variant,
    );
  }
});
