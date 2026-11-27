import { z } from "zod";

import { AgentIssueSchema } from "./AgentIssue";
import { AgentRuntimeStatusSchema } from "./AgentRuntimeStatus";
import { DesiredStateVersionSchema } from "./DesiredStateVersion";
import { ModelDownloadStatusSchema } from "./ModelDownloadStatus";

export const AgentStatusSchema = z
  .object({
    desired_slots_total: z.number(),
    desired_state_version: DesiredStateVersionSchema,
    download_status: ModelDownloadStatusSchema,
    issues: z.array(AgentIssueSchema),
    model_path: z.string().nullable(),
    runtime: AgentRuntimeStatusSchema,
    state_application_status: z.enum([
      "Applied",
      "AttemptedAndRetrying",
      "Fresh",
      "Stuck",
    ]),
    uses_chat_template_override: z.boolean(),
  })
  .strict();

export type AgentStatus = z.infer<typeof AgentStatusSchema>;
