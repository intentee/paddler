import React, { useContext } from "react";
import { Link } from "wouter";

import { type ConversationUnsupportedReason } from "@intentee/paddler-client/schemas/ConversationUnsupportedReason";
import { clusterConversationCapability } from "../clusterConversationCapability";
import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { useAgentsStream } from "../hooks/useAgentsStream";
import { matchEventSourceUpdateState } from "../matchEventSourceUpdateState";
import { ConversationPromptInput } from "./ConversationPromptInput";

import { conversationPromptStream__notice } from "./ConversationPromptStream.module.css";

function ConversationUnsupportedReasonExplanation({
  reason,
}: {
  reason: ConversationUnsupportedReason;
}) {
  switch (reason) {
    case "ModelGeneratesNoText":
      return (
        <p>
          Agents cannot hold a conversation, because their model generates no
          text. <Link href="/model">Change the model</Link> to one that
          generates text to prompt it here.
        </p>
      );
    case "ModelHasNoChatTemplate":
      return (
        <p>
          Agents cannot hold a conversation, because their model has no chat
          template. <Link href="/model">Provide a chat template</Link> on the
          model page to prompt it here.
        </p>
      );
  }
}

export function ConversationPromptStream() {
  const { managementAddr } = useContext(PaddlerConfigurationContext);
  const eventSourceUpdateState = useAgentsStream({ managementAddr });

  return matchEventSourceUpdateState(eventSourceUpdateState, {
    connected() {
      return (
        <div className={conversationPromptStream__notice}>
          Connected to the server, waiting for agents...
        </div>
      );
    },
    connectionError() {
      return (
        <div className={conversationPromptStream__notice}>
          Cannot connect to the server to find out which agents hold
          conversations. Will try to reconnect in a few seconds...
        </div>
      );
    },
    dataSnapshot({ data: { agents } }) {
      const conversationCapability = clusterConversationCapability(agents);

      switch (conversationCapability.kind) {
        case "Conversing":
          return (
            <ConversationPromptInput
              conversationFeatures={conversationCapability.conversationFeatures}
            />
          );
        case "ModelsCannotConverse":
          return (
            <div className={conversationPromptStream__notice}>
              {conversationCapability.unsupportedReasons.map(function (reason) {
                return (
                  <ConversationUnsupportedReasonExplanation
                    key={reason}
                    reason={reason}
                  />
                );
              })}
            </div>
          );
        case "NoAgentServesAModel":
          return (
            <div className={conversationPromptStream__notice}>
              No agent serves a chat model yet. Once an agent serves a model
              that holds conversations, you can prompt it here.
            </div>
          );
      }
    },
    deserializationError() {
      return (
        <div className={conversationPromptStream__notice}>
          Error deserializing agents data from the server.
        </div>
      );
    },
    initial() {
      return (
        <div className={conversationPromptStream__notice}>
          Connecting to the server...
        </div>
      );
    },
  });
}
