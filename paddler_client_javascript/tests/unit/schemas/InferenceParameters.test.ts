import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import {
  InferenceParametersSchema,
  type InferenceParameters,
} from "../../../src/schemas/InferenceParameters";
import { assertSchemaRejectsPaths } from "../../assertSchemaRejectsPaths";

const validParameters: InferenceParameters = {
  n_batch: 512,
  context_size: 4096,
  embedding_batch_size: 64,
  enable_embeddings: false,
  image_resize_to_fit: 512,
  k_cache_dtype: "F16",
  v_cache_dtype: "F16",
  min_p: 0.05,
  n_gpu_layers: -1,
  penalty_frequency: 0.5,
  penalty_last_n: 64,
  penalty_presence: 0,
  penalty_repeat: 1.1,
  pooling_type: "Mean",
  temperature: 0.7,
  top_k: 40,
  top_p: 0.9,
};

test("accepts parameters that satisfy every rule", function () {
  strictEqual(
    InferenceParametersSchema.safeParse(validParameters).success,
    true,
  );
});

test("rejects a batch larger than the context", function () {
  assertSchemaRejectsPaths({
    expectedPaths: [["n_batch"]],
    input: { ...validParameters, n_batch: 8192 },
    schema: InferenceParametersSchema,
  });
});

test("rejects penalty strengths without a penalty window", function () {
  assertSchemaRejectsPaths({
    expectedPaths: [["penalty_last_n"]],
    input: { ...validParameters, penalty_last_n: 0 },
    schema: InferenceParametersSchema,
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
    schema: InferenceParametersSchema,
  });
});
