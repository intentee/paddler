import React, { memo, useContext, useEffect, useMemo, useState } from "react";
import { scan } from "rxjs";

import { inferenceSocketClient } from "@intentee/paddler-client/inferenceSocketClient";
import { type ConversationMessage as ConversationMessageType } from "@intentee/paddler-client/schemas/ConversationMessage";
import { type InferenceServiceGenerateTokensResponse } from "@intentee/paddler-client/schemas/InferenceServiceGenerateTokensResponse";
import { PromptContext } from "../contexts/PromptContext";
import { PromptImageContext } from "../contexts/PromptImageContext";
import { PromptThinkingContext } from "../contexts/PromptThinkingContext";
import { ConversationMessage } from "./ConversationMessage";

interface Message {
  errors: Array<{
    code: number;
    description: string;
  }>;
  isEmpty: boolean;
  isThinking: boolean;
  response: string;
  thoughts: string;
}

const defaultMessage: Message = Object.freeze({
  errors: [],
  isEmpty: true,
  isThinking: false,
  response: "",
  thoughts: "",
});

function buildUserMessage(
  submittedPrompt: string,
  submittedImageDataUri: null | string,
): ConversationMessageType {
  if (submittedImageDataUri) {
    return {
      role: "user",
      content: [
        {
          type: "image_url",
          image_url: { url: submittedImageDataUri },
        },
        {
          type: "text",
          text: submittedPrompt,
        },
      ],
    };
  }

  return {
    role: "user",
    content: submittedPrompt,
  };
}

export const ConversationMessagePromptGeneratedTokens = memo(
  function ConversationMessagePromptGeneratedTokens({
    webSocket,
  }: {
    webSocket: WebSocket;
  }) {
    const { submittedPrompt, version } = useContext(PromptContext);
    const { submittedImageDataUri } = useContext(PromptImageContext);
    const { submittedIsThinkingEnabled } = useContext(PromptThinkingContext);
    const [message, setMessage] = useState<Message>(defaultMessage);

    const socketClient = useMemo(
      function () {
        return inferenceSocketClient({ webSocket });
      },
      [webSocket],
    );

    useEffect(
      function () {
        if (!submittedPrompt || !submittedPrompt.trim()) {
          return;
        }

        const subscription = socketClient
          .continueConversation({
            enableThinking: submittedIsThinkingEnabled,
            messages: [
              {
                role: "system",
                content:
                  "You are a helpful assistant. Give engaging, short, precise answers. Be friendly, supportive, use emojis.",
              },
              {
                role: "assistant",
                content: "Hello! How can I help you today?",
              },
              buildUserMessage(submittedPrompt, submittedImageDataUri),
            ],
          })
          .pipe(
            scan(function (
              message: Message,
              chunk: InferenceServiceGenerateTokensResponse,
            ) {
              if (chunk.error) {
                return Object.freeze({
                  ...message,
                  errors: [...message.errors, chunk.error],
                  isEmpty: false,
                });
              }

              if (chunk.done) {
                return Object.freeze({
                  errors: message.errors,
                  isEmpty: false,
                  isThinking: false,
                  response: message.response,
                  thoughts: message.thoughts,
                });
              }

              if (null === chunk.token) {
                return message;
              }

              if ("reasoning" === chunk.tokenKind) {
                return Object.freeze({
                  errors: message.errors,
                  isEmpty: false,
                  isThinking: true,
                  response: message.response,
                  thoughts: `${message.thoughts}${chunk.token}`,
                });
              }

              if ("tool_call" === chunk.tokenKind) {
                return Object.freeze({
                  ...message,
                  isEmpty: false,
                  isThinking: false,
                });
              }

              return Object.freeze({
                errors: message.errors,
                isEmpty: false,
                isThinking: false,
                response: `${message.response}${chunk.token}`,
                thoughts: message.thoughts,
              });
            }, defaultMessage),
          )
          .subscribe(setMessage);

        return function () {
          subscription.unsubscribe();
        };
      },
      [
        socketClient,
        setMessage,
        submittedImageDataUri,
        submittedIsThinkingEnabled,
        submittedPrompt,
        version,
      ],
    );

    if (message.isEmpty) {
      if (submittedPrompt) {
        return (
          <ConversationMessage
            author="AI"
            errors={message.errors}
            isThinking={true}
            response={message.response}
            thoughts={message.thoughts}
          />
        );
      }

      return;
    }

    return (
      <ConversationMessage
        author="AI"
        errors={message.errors}
        isThinking={message.isThinking}
        response={message.response}
        thoughts={message.thoughts}
      />
    );
  },
);
