import { z } from "zod";
import { AgentIssueModelPathSchema } from "./AgentIssueModelPath";

export const HuggingFaceDownloadLockSchema = z.object({
  lock_path: z.string(),
  model_path: AgentIssueModelPathSchema,
});

export type HuggingFaceDownloadLock = z.infer<typeof HuggingFaceDownloadLockSchema>;
