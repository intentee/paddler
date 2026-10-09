import { z } from "zod";

import { ChatTemplateSchema } from "./ChatTemplate";
import { MultimodalSettingsSchema } from "./MultimodalSettings";
import { SamplingParametersSchema } from "./SamplingParameters";

export const BalancerTextGenerationSettingsSchema = z
  .object({
    chat_template_override: ChatTemplateSchema.nullable(),
    multimodal: MultimodalSettingsSchema,
    sampling_parameters: SamplingParametersSchema,
    use_chat_template_override: z.boolean(),
  })
  .strict();

export type BalancerTextGenerationSettings = z.infer<
  typeof BalancerTextGenerationSettingsSchema
>;
