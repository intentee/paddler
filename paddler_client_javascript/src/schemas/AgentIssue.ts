import { z } from "zod";

import { AgentIssueModelPathSchema } from "./AgentIssueModelPath";
import { HuggingFaceDownloadLockSchema } from "./HuggingFaceDownloadLock";

export const AgentIssueSchema = z.union([
  z.object({
    CacheCannotAcquireLock: AgentIssueModelPathSchema,
  }),
  z.object({
    CacheDirectoryIsNotWritable: AgentIssueModelPathSchema,
  }),
  z.object({
    CacheStorageIsFull: AgentIssueModelPathSchema,
  }),
  z.object({
    ChatTemplateDoesNotCompile: z.object({
      error: z.string(),
      model_path: AgentIssueModelPathSchema,
      template_content: z.string(),
    }),
  }),
  z.object({
    DownloadServerDeniedAccess: AgentIssueModelPathSchema,
  }),
  z.object({
    DownloadServerErrored: AgentIssueModelPathSchema,
  }),
  z.object({
    DownloadServerIsUnreachable: AgentIssueModelPathSchema,
  }),
  z.object({
    DownloadServerRejectedRequest: AgentIssueModelPathSchema,
  }),
  z.object({
    DownloadInterrupted: AgentIssueModelPathSchema,
  }),
  z.object({
    DownloadUrlIsMalformed: AgentIssueModelPathSchema,
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
    ModelArchitectureUnsupportedForDecisions: z.object({
      architecture: z.string(),
      model_path: AgentIssueModelPathSchema,
    }),
  }),
  z.object({
    ModelCacheIsCorrupted: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelCannotBeLoaded: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelDoesNotExistAtUrl: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelFileDoesNotExist: AgentIssueModelPathSchema,
  }),
  z.object({
    MultimodalProjectionCannotBeLoaded: AgentIssueModelPathSchema,
  }),
  z.object({
    PointerHeadCannotBeLoaded: AgentIssueModelPathSchema,
  }),
  z.object({
    PointerHeadIncompatibleWithModel: z.object({
      incompatibility: z.union([
        z.object({
          DelimiterIsNotAControlToken: z.object({ delimiter: z.string() }),
        }),
        z.object({
          HiddenSizeMismatch: z.object({
            model_hidden_size: z.number(),
            pointer_head_hidden_size: z.number(),
          }),
        }),
      ]),
      pointer_head_path: AgentIssueModelPathSchema,
    }),
  }),
  z.object({
    SlotCannotStart: z.object({
      error: z.string(),
      slot_index: z.number(),
    }),
  }),
  z.object({
    SlotsInsufficientForDecisions: z.object({
      desired_slots: z.number(),
      required_slots: z.number(),
    }),
  }),
  z.object({
    UnableToFindChatTemplate: AgentIssueModelPathSchema,
  }),
]);

export type AgentIssue = z.infer<typeof AgentIssueSchema>;
