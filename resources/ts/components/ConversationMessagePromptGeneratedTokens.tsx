import React, { memo, useContext, useEffect, useMemo, useState } from "react";
import { scan } from "rxjs";

import { PromptContext } from "../contexts/PromptContext";
import { PromptImageContext } from "../contexts/PromptImageContext";
import { PromptThinkingContext } from "../contexts/PromptThinkingContext";
import { type ConversationMessage as ConversationMessageType } from "../ConversationMessage.type";
import { InferenceSocketClient } from "../InferenceSocketClient";
import { type InferenceServiceGenerateTokensResponse } from "../schemas/InferenceServiceGenerateTokensResponse";
import { ConversationMessage } from "./ConversationMessage";

interface Message {
  errors: Array<{
    code: number;
    description: string;
  }>;
  isEmpty: boolean;
  response: string;
  thoughts: string;
}

const defaultMessage: Message = Object.freeze({
  errors: [],
  isEmpty: true,
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

    const inferenceSocketClient = useMemo(
      function () {
        return InferenceSocketClient({ webSocket });
      },
      [webSocket],
    );

    useEffect(
      function () {
        if (!submittedPrompt || !submittedPrompt.trim()) {
          return;
        }

        const subscription = inferenceSocketClient
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
              {
                done,
                error,
                thinking_token,
                token,
              }: InferenceServiceGenerateTokensResponse,
            ) {
              if (error) {
                return Object.freeze({
                  ...message,
                  errors: [...message.errors, error],
                  isEmpty: false,
                });
              }

              if (done) {
                return Object.freeze({
                  errors: message.errors,
                  isEmpty: false,
                  response: message.response,
                  thoughts: message.thoughts,
                });
              }

              if (thinking_token) {
                return Object.freeze({
                  errors: message.errors,
                  isEmpty: false,
                  response: message.response,
                  thoughts: `${message.thoughts}${thinking_token}`,
                });
              }

              if (token) {
                return Object.freeze({
                  errors: message.errors,
                  isEmpty: false,
                  response: `${message.response}${token}`,
                  thoughts: message.thoughts,
                });
              }

              return message;
            }, defaultMessage),
          )
          .subscribe(setMessage);

        return function () {
          subscription.unsubscribe();
        };
      },
      [
        inferenceSocketClient,
        setMessage,
        submittedImageDataUri,
        submittedIsThinkingEnabled,
        submittedPrompt,
        version,
      ],
    );

    if (message.isEmpty) {
      return;
    }

    return (
      <ConversationMessage
        author="AI"
        errors={message.errors}
        response={message.response}
        thoughts={message.thoughts}
      />
    );
  },
);
