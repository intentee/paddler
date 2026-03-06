import { z } from "zod";

export const AgentIssueSeveritySchema = z.enum(["Error", "Warning"]);

export type AgentIssueSeverity = z.infer<typeof AgentIssueSeveritySchema>;

export const AgentIssueTypeSchema = z.union([
  z.object({
    ChatTemplateDoesNotCompile: z.object({
      error: z.string(),
      template_content: z.string(),
    }),
  }),
  z.object({
    HuggingFaceCannotAcquireLock: z.string(),
  }),
  z.object({
    HuggingFaceModelDoesNotExist: z.string(),
  }),
  z.object({
    ModelCannotBeLoaded: z.string(),
  }),
  z.object({
    ModelFileDoesNotExist: z.string(),
  }),
  z.object({
    SlotCannotStart: z.object({
      error: z.string(),
      slot_index: z.number(),
    }),
  }),
  z.object({
    UnableToFindChatTemplate: z.string(),
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
