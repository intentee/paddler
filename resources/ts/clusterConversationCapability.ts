import { type Agent } from "@intentee/paddler-client/schemas/Agent";
import { type ConversationUnsupportedReason } from "@intentee/paddler-client/schemas/ConversationUnsupportedReason";
import { type ModelCapabilities } from "@intentee/paddler-client/schemas/ModelCapabilities";

import { type ConversationFeatures } from "./ConversationFeatures";
import { servedModelCapabilities } from "./servedModelCapabilities";

export type ClusterConversationCapability =
  | { conversationFeatures: ConversationFeatures; kind: "Conversing" }
  | {
      kind: "ModelsCannotConverse";
      unsupportedReasons: Array<ConversationUnsupportedReason>;
    }
  | { kind: "NoAgentServesAModel" };

function conversationFeaturesOf({
  conversation,
  input_media,
}: ModelCapabilities): Array<ConversationFeatures> {
  if ("Supported" in conversation) {
    return [
      {
        acceptsAudio: input_media.audio,
        acceptsImages: input_media.image,
        supportsSystemRole: conversation.Supported.system_role,
        supportsThinking: conversation.Supported.thinking,
      },
    ];
  }

  return [];
}

function conversationUnsupportedReasonsOf({
  conversation,
}: ModelCapabilities): Array<ConversationUnsupportedReason> {
  if ("Unsupported" in conversation) {
    return [conversation.Unsupported];
  }

  return [];
}

export function clusterConversationCapability(
  agents: Array<Agent>,
): ClusterConversationCapability {
  const servedCapabilities = servedModelCapabilities(agents);

  if (servedCapabilities.length < 1) {
    return { kind: "NoAgentServesAModel" };
  }

  const conversingAgentsFeatures = servedCapabilities.flatMap(
    conversationFeaturesOf,
  );

  if (conversingAgentsFeatures.length < 1) {
    return {
      kind: "ModelsCannotConverse",
      unsupportedReasons: Array.from(
        new Set(servedCapabilities.flatMap(conversationUnsupportedReasonsOf)),
      ),
    };
  }

  return {
    conversationFeatures: {
      acceptsAudio: conversingAgentsFeatures.every(function ({ acceptsAudio }) {
        return acceptsAudio;
      }),
      acceptsImages: conversingAgentsFeatures.every(function ({
        acceptsImages,
      }) {
        return acceptsImages;
      }),
      supportsSystemRole: conversingAgentsFeatures.every(function ({
        supportsSystemRole,
      }) {
        return supportsSystemRole;
      }),
      supportsThinking: conversingAgentsFeatures.every(function ({
        supportsThinking,
      }) {
        return supportsThinking;
      }),
    },
    kind: "Conversing",
  };
}
