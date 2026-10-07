import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import {
  SamplingParametersSchema,
  type SamplingParameters,
} from "../../../src/schemas/SamplingParameters";
import { assertSchemaRejectsPaths } from "../../assertSchemaRejectsPaths";

const validParameters: SamplingParameters = {
  min_p: 0.05,
  penalty_frequency: 0.5,
  penalty_last_n: 64,
  penalty_presence: 0,
  penalty_repeat: 1.1,
  temperature: 0.7,
  top_k: 40,
  top_p: 0.9,
};

test("accepts sampling parameters that satisfy every rule", function () {
  strictEqual(
    SamplingParametersSchema.safeParse(validParameters).success,
    true,
  );
});

test("rejects penalty strengths without a penalty window", function () {
  assertSchemaRejectsPaths({
    expectedPaths: [["penalty_last_n"]],
    input: { ...validParameters, penalty_last_n: 0 },
    schema: SamplingParametersSchema,
  });
});

test("rejects a penalty window without penalty strengths", function () {
  assertSchemaRejectsPaths({
    expectedPaths: [["penalty_last_n"]],
    input: {
      ...validParameters,
      penalty_frequency: 0,
      penalty_repeat: 1,
    },
    schema: SamplingParametersSchema,
  });
});
