import { z } from "zod";

import { AgentDesiredModelSchema } from "./AgentDesiredModel";
import { BalancerTextGenerationSettingsSchema } from "./BalancerTextGenerationSettings";
import { DecisionSettingsSchema } from "./DecisionSettings";
import { EmbeddingParametersSchema } from "./EmbeddingParameters";
import { InferenceModeSchema } from "./InferenceMode";
import { ModelRuntimeParametersSchema } from "./ModelRuntimeParameters";

export const BalancerDesiredStateSchema = z
  .object({
    decision: DecisionSettingsSchema,
    embeddings: EmbeddingParametersSchema,
    inference_mode: InferenceModeSchema,
    model: AgentDesiredModelSchema,
    model_runtime_parameters: ModelRuntimeParametersSchema,
    text_generation: BalancerTextGenerationSettingsSchema,
  })
  .strict();

export type BalancerDesiredState = z.infer<typeof BalancerDesiredStateSchema>;
