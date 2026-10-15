import { z } from "zod";

import { AgentDesiredModelSchema } from "./AgentDesiredModel";

export const DecisionSettingsSchema = z
  .object({
    pointer_head: AgentDesiredModelSchema,
  })
  .strict();

export type DecisionSettings = z.infer<typeof DecisionSettingsSchema>;
