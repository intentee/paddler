import { z } from "zod";

import { AgentIssueModelPathSchema } from "./AgentIssueModelPath";
import { HuggingFaceDownloadLockSchema } from "./HuggingFaceDownloadLock";

export const IssueSeveritySchema = z.enum(["Error", "Warning"]);

export type IssueSeverity = z.infer<typeof IssueSeveritySchema>;

export const IssueTypeSchema = z.union([
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

export type IssueType = z.infer<typeof IssueTypeSchema>;

export const AgentIssueSchema = z
  .object({
    severity: IssueSeveritySchema,
    type: IssueTypeSchema,
  })
  .strict();

export type AgentIssue = z.infer<typeof AgentIssueSchema>;
