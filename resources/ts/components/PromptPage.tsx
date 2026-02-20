import React, { useContext } from "react";

import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { PromptContext } from "../contexts/PromptContext";
import { useWebSocket } from "../hooks/useWebSocket";
import { matchWebSocketState } from "../matchWebSocketState";
import { webSocketProtocol } from "../webSocketProtocol";
import { ConversationMessage } from "./ConversationMessage";
import { ConversationMessagePromptGeneratedTokens } from "./ConversationMessagePromptGeneratedTokens";
import { ConversationPromptInput } from "./ConversationPromptInput";
import { FloatingStatus } from "./FloatingStatus";

import {
  promptPage,
  promptPage__messages,
  promptPage__promptForm,
} from "./PromptPage.module.css";

export function PromptPage() {
  const { inferenceAddr } = useContext(PaddlerConfigurationContext);
  const { submittedPrompt } = useContext(PromptContext);
  const webSocketState = useWebSocket({
    endpoint: `${webSocketProtocol(window.location.protocol)}//${inferenceAddr}/api/v1/inference_socket`,
  });

  return matchWebSocketState(webSocketState, {
    connected({ webSocket }) {
      return (
        <div className={promptPage}>
          <div className={promptPage__messages}>
            {submittedPrompt && (
              <ConversationMessage
                aiMessage=""
                author="You"
                errors={[]}
                isThinking={false}
                thoughts=""
                userMessage={submittedPrompt}
              />
            )}
            <ConversationMessagePromptGeneratedTokens webSocket={webSocket} />
          </div>
          <div className={promptPage__promptForm}>
            <ConversationPromptInput />
          </div>
        </div>
      );
    },
    connecting() {
      return (
        <FloatingStatus>Connecting to the inference server...</FloatingStatus>
      );
    },
    connectionClosed() {
      return (
        <FloatingStatus>
          Connection to the inference server closed. Will try to reconnect...
        </FloatingStatus>
      );
    },
    connectionError() {
      return (
        <FloatingStatus>
          Cannot connect to the inference server. Will try again in a moment...
        </FloatingStatus>
      );
    },
  });
}
