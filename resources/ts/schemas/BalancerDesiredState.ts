import { z } from "zod";

import { AgentDesiredModelSchema } from "./AgentDesiredModel";
import { ChatTemplateSchema } from "./ChatTemplate";
import { InferenceParametersSchema } from "./InferenceParameters";

export const BalancerDesiredStateSchema = z
  .object({
    chat_template_override: ChatTemplateSchema.nullable(),
    inference_parameters: InferenceParametersSchema,
    model: AgentDesiredModelSchema,
    multimodal_projection: AgentDesiredModelSchema,
    use_chat_template_override: z.boolean(),
  })
  .strict();

export type BalancerDesiredState = z.infer<typeof BalancerDesiredStateSchema>;
