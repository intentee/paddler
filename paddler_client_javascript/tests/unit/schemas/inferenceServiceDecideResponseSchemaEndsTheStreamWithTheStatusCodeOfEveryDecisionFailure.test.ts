import { deepStrictEqual, strictEqual } from "node:assert/strict";
import { test } from "node:test";

import { InferenceServiceDecideResponseSchema } from "../../../src/schemas/InferenceServiceDecideResponse";
import { decisionResponse } from "./decisionResponse";

const decisionFailureCodes = [
  { code: 500, variant: "AgentRuntimeFailed" },
  { code: 500, variant: "BatchAssemblyFailed" },
  { code: 503, variant: "DecisionsNotServed" },
  { code: 500, variant: "DecodeFailed" },
  { code: 500, variant: "HiddenStateUnavailable" },
  { code: 400, variant: "InputTokenizationFailed" },
  { code: 500, variant: "KvCacheCopyFailed" },
  { code: 500, variant: "KvCacheRemovalFailed" },
  { code: 503, variant: "ModelNotLoaded" },
  { code: 503, variant: "SchedulerUnavailable" },
];

test("InferenceServiceDecideResponseSchema ends the stream with the status code of every decision failure", function () {
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
