import { z } from "zod";

import { AgentDesiredModelSchema } from "./AgentDesiredModel";

export const MultimodalSettingsSchema = z
  .object({
    image_resize_to_fit: z.number().int().min(1),
    projection: AgentDesiredModelSchema,
  })
  .strict();

export type MultimodalSettings = z.infer<typeof MultimodalSettingsSchema>;
