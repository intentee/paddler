import { z } from "zod";

import { AgentIssueModelCapabilityFailureSchema } from "./AgentIssueModelCapabilityFailure";
import { AgentIssueModelPathSchema } from "./AgentIssueModelPath";
import { ChatTemplateSourceSchema } from "./ChatTemplateSource";
import { HuggingFaceDownloadLockSchema } from "./HuggingFaceDownloadLock";
import { InferenceCapabilitySchema } from "./InferenceCapability";

export const AgentIssueSchema = z.union([
  z.object({
    AgentModelUriIsNotAnAbsoluteLocalPath: AgentIssueModelPathSchema,
  }),
  z.object({
    AgentModelUriIsNotCanonical: z.object({
      canonical_uri: z.string(),
      model_path: AgentIssueModelPathSchema,
    }),
  }),
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
    ChatTemplateCapabilitiesUnreadable: AgentIssueModelCapabilityFailureSchema,
  }),
  z.object({
    ChatTemplateDoesNotCompile: z.object({
      error: z.string(),
      model_path: AgentIssueModelPathSchema,
      template_source: ChatTemplateSourceSchema,
    }),
  }),
  z.object({
    DecisionModelCannotBeServed: AgentIssueModelCapabilityFailureSchema,
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
    EmbeddingPoolingProducesNoVectors: AgentIssueModelPathSchema,
  }),
  z.object({
    HuggingFaceCannotAcquireLock: HuggingFaceDownloadLockSchema,
  }),
  z.object({
    HuggingFaceDownloadFailed: z.object({
      failure: z.union([
        z.literal("CacheIoFailed"),
        z.literal("HubConnectionFailed"),
        z.object({
          HubRespondedWithStatus: z.object({ status: z.number() }),
        }),
        z.literal("HubResponseMalformed"),
        z.literal("UnexpectedHubClientFailure"),
        z.literal("XetTransferFailed"),
      ]),
      model_path: AgentIssueModelPathSchema,
    }),
  }),
  z.object({
    HuggingFaceModelDoesNotExist: AgentIssueModelPathSchema,
  }),
  z.object({
    HuggingFaceModelUriIsMalformed: AgentIssueModelPathSchema,
  }),
  z.object({
    HuggingFacePermissions: AgentIssueModelPathSchema,
  }),
  z.object({
    LocalModelFileCannotBeChecked: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelCacheIsCorrupted: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelCannotBeLoaded: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelDeclaresAnUnknownPoolingType: AgentIssueModelCapabilityFailureSchema,
  }),
  z.object({
    ModelDoesNotExistAtUrl: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelFileDoesNotExist: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelTrainingContextIsUnrepresentable: AgentIssueModelPathSchema,
  }),
  z.object({
    ModelUriIsUnparseable: AgentIssueModelPathSchema,
  }),
  z.object({
    MultimodalProjectionCannotBeLoaded: AgentIssueModelPathSchema,
  }),
  z.object({
    SlotCannotStart: z.object({
      error: z.string(),
      inference_capability: InferenceCapabilitySchema,
      slot_index: z.number(),
    }),
  }),
  z.object({
    SlotsInsufficientForDecisions: z.object({
      desired_slots: z.number(),
      required_slots: z.number(),
    }),
  }),
]);

export type AgentIssue = z.infer<typeof AgentIssueSchema>;
