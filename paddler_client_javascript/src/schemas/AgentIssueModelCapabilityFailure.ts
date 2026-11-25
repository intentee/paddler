import { z } from "zod";

import { AgentIssueModelPathSchema } from "./AgentIssueModelPath";

export const AgentIssueModelCapabilityFailureSchema = z.object({
  error: z.string(),
  model_path: AgentIssueModelPathSchema,
});

export type AgentIssueModelCapabilityFailure = z.infer<
  typeof AgentIssueModelCapabilityFailureSchema
>;
