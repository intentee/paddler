import { strictEqual } from "node:assert/strict";
import { test } from "node:test";

import {
  ModelRuntimeParametersSchema,
  type ModelRuntimeParameters,
} from "../../../src/schemas/ModelRuntimeParameters";
import { assertSchemaRejectsPaths } from "../../assertSchemaRejectsPaths";

const validParameters: ModelRuntimeParameters = {
  context_size: 4096,
  k_cache_dtype: "F16",
  n_batch: 512,
  n_gpu_layers: -1,
  v_cache_dtype: "F16",
};

test("accepts model runtime parameters that satisfy every rule", function () {
  strictEqual(
    ModelRuntimeParametersSchema.safeParse(validParameters).success,
    true,
  );
});

test("rejects a batch larger than the context", function () {
  assertSchemaRejectsPaths({
    expectedPaths: [["n_batch"]],
    input: { ...validParameters, n_batch: 8192 },
    schema: ModelRuntimeParametersSchema,
  });
});
