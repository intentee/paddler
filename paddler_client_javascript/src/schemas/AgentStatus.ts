import { z } from "zod";

import { AgentIssueSchema } from "./AgentIssue";
import { ModelDownloadStatusSchema } from "./ModelDownloadStatus";

export const AgentStatusSchema = z
  .object({
    desired_slots_total: z.number(),
    download_status: ModelDownloadStatusSchema,
    issues: z.array(AgentIssueSchema),
    model_path: z.string().nullable(),
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

export type AgentStatus = z.infer<typeof AgentStatusSchema>;
