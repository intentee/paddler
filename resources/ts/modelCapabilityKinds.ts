import { type ChatTemplateCapabilities } from "@intentee/paddler-client/schemas/ChatTemplateCapabilities";
import { type ConversationUnsupportedReason } from "@intentee/paddler-client/schemas/ConversationUnsupportedReason";
import { type DecisionsUnsupportedReason } from "@intentee/paddler-client/schemas/DecisionsUnsupportedReason";
import { type EmbeddingsUnsupportedReason } from "@intentee/paddler-client/schemas/EmbeddingsUnsupportedReason";
import { type InputMediaCapabilities } from "@intentee/paddler-client/schemas/InputMediaCapabilities";
import { type TextGenerationUnsupportedReason } from "@intentee/paddler-client/schemas/TextGenerationUnsupportedReason";

import { type ModelCapabilityKind } from "./ModelCapabilityKind";

const toolCallingDescriptions: Record<
  ChatTemplateCapabilities["tool_calling"],
  string
> = {
  OneCallPerTurn: "one call per turn",
  ParallelCalls: "parallel calls",
  Unsupported: "no",
};

const conversationUnsupportedDescriptions: Record<
  ConversationUnsupportedReason,
  string
> = {
  ModelGeneratesNoText:
    "Cannot hold conversations: the model generates no text.",
  ModelHasNoChatTemplate:
    "Cannot hold conversations: the model has no chat template.",
};

const textGenerationUnsupportedDescriptions: Record<
  TextGenerationUnsupportedReason,
  string
> = {
  AttentionIsNotCausal:
    "Cannot generate text: the attention of the model is not causal.",
  EncoderDecoderArchitecture:
    "Cannot generate text: the model has an encoder-decoder architecture.",
  NoTextOutput: "Cannot generate text: the model outputs no text.",
};

const embeddingsUnsupportedDescriptions: Record<
  EmbeddingsUnsupportedReason,
  string
> = {
  ModelDeclaresNoVectorPooling:
    "Cannot generate embeddings: the model declares no pooling that produces embedding vectors.",
};

const decisionsUnsupportedDescriptions: Record<
  DecisionsUnsupportedReason,
  string
> = {
  DecisionTypeUnsupportedByLlamaCpp:
    "Cannot decide: llama.cpp does not support the decision type the model declares.",
  ModelIsNotADecisionModel: "Cannot decide: the model is not a decision model.",
};

function yesOrNo(isSupported: boolean): string {
  return isSupported ? "yes" : "no";
}

function conversationDescription(
  { system_role, thinking, tool_calling }: ChatTemplateCapabilities,
  { audio, image }: InputMediaCapabilities,
): string {
  return `Holds conversations through its chat template. System messages: ${yesOrNo(system_role)}. Thinking: ${yesOrNo(thinking)}. Tool calls: ${toolCallingDescriptions[tool_calling]}. Images: ${yesOrNo(image)}. Audio: ${yesOrNo(audio)}.`;
}

export const modelCapabilityKinds: ReadonlyArray<ModelCapabilityKind> = [
  {
    label: "Conversation",
    statusOf({ conversation, input_media }) {
      if ("Supported" in conversation) {
        return {
          description: conversationDescription(
            conversation.Supported,
            input_media,
          ),
          isSupported: true,
        };
      }

      return {
        description:
          conversationUnsupportedDescriptions[conversation.Unsupported],
        isSupported: false,
      };
    },
  },
  {
    label: "Text generation",
    statusOf({ text_generation }) {
      if (text_generation === "Supported") {
        return {
          description: "Continues raw prompts.",
          isSupported: true,
        };
      }

      return {
        description:
          textGenerationUnsupportedDescriptions[text_generation.Unsupported],
        isSupported: false,
      };
    },
  },
  {
    label: "Embeddings",
    statusOf({ embeddings }) {
      if (embeddings === "Supported") {
        return {
          description: "Generates embeddings.",
          isSupported: true,
        };
      }

      return {
        description: embeddingsUnsupportedDescriptions[embeddings.Unsupported],
        isSupported: false,
      };
    },
  },
  {
    label: "Decisions",
    statusOf({ decisions }) {
      if ("Supported" in decisions) {
        return {
          description: decisions.Supported.reads_images
            ? "Decides questions about text and images."
            : "Decides questions about text.",
          isSupported: true,
        };
      }

      return {
        description: decisionsUnsupportedDescriptions[decisions.Unsupported],
        isSupported: false,
      };
    },
  },
];
