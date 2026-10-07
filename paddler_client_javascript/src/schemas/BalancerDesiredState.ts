import { z } from "zod";

import { AgentDesiredModelSchema } from "./AgentDesiredModel";
import { BalancerInferenceSettingsSchema } from "./BalancerInferenceSettings";
import { ModelRuntimeParametersSchema } from "./ModelRuntimeParameters";

export const BalancerDesiredStateSchema = z
  .object({
    inference_settings: BalancerInferenceSettingsSchema,
    model: AgentDesiredModelSchema,
    model_runtime_parameters: ModelRuntimeParametersSchema,
  })
  .strict();

export type BalancerDesiredState = z.infer<typeof BalancerDesiredStateSchema>;
