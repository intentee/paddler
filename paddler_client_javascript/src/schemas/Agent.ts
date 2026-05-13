import { z } from "zod";

import { AgentIssueSchema } from "./AgentIssue";

export const AgentSchema = z
  .object({
    desired_slots_total: z.number(),
    download_current: z.number(),
    download_filename: z.string().nullable(),
    download_total: z.number(),
    id: z.string(),
    issues: z.array(AgentIssueSchema),
    model_path: z.string().nullable(),
    name: z.string().nullable(),
    slots_processing: z.number(),
    slots_total: z.number(),
    state_application_status: z.enum([
      "Applied",
      "AttemptedAndNotAppliable",
      "AttemptedAndRetrying",
      "Fresh",
      "Stuck",
    ]),
    uses_chat_template_override: z.boolean(),
  })
  .strict();

export type Agent = z.infer<typeof AgentSchema>;
