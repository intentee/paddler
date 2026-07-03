import React, { useContext } from "react";

import { webSocketProtocol } from "@intentee/paddler-client/webSocketProtocol";
import { PaddlerConfigurationContext } from "../contexts/PaddlerConfigurationContext";
import { PromptContext } from "../contexts/PromptContext";
import { useWebSocket } from "../hooks/useWebSocket";
import { matchWebSocketState } from "../matchWebSocketState";
import { ConversationMessage } from "./ConversationMessage";
import { ConversationMessagePromptGeneratedTokens } from "./ConversationMessagePromptGeneratedTokens";
import { ConversationPromptInput } from "./ConversationPromptInput";
import { FloatingStatus } from "./FloatingStatus";
import { TokenGenerationDisabledContextProvider } from "./TokenGenerationDisabledContextProvider";

import {
  promptPage,
  promptPage__messages,
  promptPage__promptForm,
} from "./PromptPage.module.css";

export function PromptPage() {
  const { inferenceAddr } = useContext(PaddlerConfigurationContext);
  const { submittedPrompt } = useContext(PromptContext);
  const { socketState, webSocket } = useWebSocket({
    endpoint: `${webSocketProtocol(window.location.protocol)}//${inferenceAddr}/api/v1/inference_socket`,
  });

  return (
    <TokenGenerationDisabledContextProvider webSocket={webSocket}>
      {matchWebSocketState(socketState, {
        connected({ webSocket }) {
          return (
            <div className={promptPage}>
              <div className={promptPage__messages}>
                {submittedPrompt && (
                  <ConversationMessage
                    author="You"
                    errors={[]}
                    isThinking={false}
                    response={submittedPrompt}
                    thoughts=""
                  />
                )}
                <ConversationMessagePromptGeneratedTokens
                  webSocket={webSocket}
                />
              </div>
              <div className={promptPage__promptForm}>
                <ConversationPromptInput />
              </div>
            </div>
          );
        },
        connecting() {
          return (
            <FloatingStatus>
              Connecting to the inference server...
            </FloatingStatus>
          );
        },
        connectionClosed() {
          return (
            <FloatingStatus>
              Connection to the inference server closed. Will try to
              reconnect...
            </FloatingStatus>
          );
        },
        connectionError() {
          return (
            <FloatingStatus>
              Cannot connect to the inference server. Will try again in a
              moment...
            </FloatingStatus>
          );
        },
      })}
    </TokenGenerationDisabledContextProvider>
  );
}
