import { z } from "zod";

import { ChatTemplateSourceSchema } from "./ChatTemplateSource";
import { MultimodalSettingsSchema } from "./MultimodalSettings";
import { SamplingParametersSchema } from "./SamplingParameters";

export const BalancerTextGenerationSettingsSchema = z
  .object({
    chat_template_source: ChatTemplateSourceSchema,
    multimodal: MultimodalSettingsSchema,
    sampling_parameters: SamplingParametersSchema,
  })
  .strict();

export type BalancerTextGenerationSettings = z.infer<
  typeof BalancerTextGenerationSettingsSchema
>;
