import { z } from "zod";

import { AgentDesiredModelSchema } from "./AgentDesiredModel";
import { BalancerTextGenerationSettingsSchema } from "./BalancerTextGenerationSettings";
import { EmbeddingParametersSchema } from "./EmbeddingParameters";
import { ModelRuntimeParametersSchema } from "./ModelRuntimeParameters";

export const BalancerDesiredStateSchema = z
  .object({
    embeddings: EmbeddingParametersSchema,
    model: AgentDesiredModelSchema,
    model_runtime_parameters: ModelRuntimeParametersSchema,
    text_generation: BalancerTextGenerationSettingsSchema,
  })
  .strict();

export type BalancerDesiredState = z.infer<typeof BalancerDesiredStateSchema>;
