import { z } from "zod";

export const AgentIssueModelPathSchema = z.object({
  model_path: z.string(),
});

export type AgentIssueModelPath = z.infer<typeof AgentIssueModelPathSchema>;
