import { z } from "zod";

import { AgentIssueModelPathSchema } from "./AgentIssueModelPath";
import { HuggingFaceDownloadLockSchema } from "./HuggingFaceDownloadLock";

export const AgentIssueSeveritySchema = z.enum(["Error", "Warning"]);

export type AgentIssueSeverity = z.infer<typeof AgentIssueSeveritySchema>;

export const AgentIssueTypeSchema = z.union([
  z.object({
    ChatTemplateDoesNotCompile: z.object({
      error: z.string(),
      model_path: AgentIssueModelPathSchema,
      template_content: z.string(),
    }),
  }),
  z.object({
    HuggingFaceCannotAcquireLock: HuggingFaceDownloadLockSchema,
  }),
  z.object({
    HuggingFaceModelDoesNotExist: AgentIssueModelPathSchema,
  }),
  z.object({
    HuggingFacePermissions: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelCannotBeLoaded: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelFileDoesNotExist: AgentIssueModelPathSchema,
  }),
  z.object({
    MultimodalProjectionCannotBeLoaded: AgentIssueModelPathSchema,
  }),
  z.object({
    SlotCannotStart: z.object({
      error: z.string(),
      slot_index: z.number(),
    }),
  }),
  z.object({
    UnableToFindChatTemplate: AgentIssueModelPathSchema,
  }),
]);

export type AgentIssueType = z.infer<typeof AgentIssueTypeSchema>;

export const AgentIssueSchema = z
  .object({
    severity: AgentIssueSeveritySchema,
    type: AgentIssueTypeSchema,
  })
  .strict();

export type AgentIssue = z.infer<typeof AgentIssueSchema>;
