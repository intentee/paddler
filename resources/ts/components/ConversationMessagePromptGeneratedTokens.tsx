import React, { memo, useContext, useEffect, useMemo, useState } from "react";
import Markdown from "react-markdown";
import { scan } from "rxjs";

import { PromptContext } from "../contexts/PromptContext";
import { InferenceSocketClient } from "../InferenceSocketClient";
import { type InferenceServiceGenerateTokensResponse } from "../schemas/InferenceServiceGenerateTokensResponse";
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

export const ConversationMessagePromptGeneratedTokens = memo(
  function ConversationMessagePromptGeneratedTokens({
    webSocket,
  }: {
    webSocket: WebSocket;
  }) {
    const { submittedPrompt, version } = useContext(PromptContext);
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
            conversation_history: [
              {
                role: "user",
                content:
                  "You are a helpful assistant. Give engaging, short, precise answers. Be friendly, supportive, use emojis.",
              },
              {
                role: "assistant",
                content: "Hello! How can I help you today?",
              },
              { role: "user", content: submittedPrompt },
            ],
          })
          .pipe(
            scan(function (
              message: Message,
              { done, error, token }: InferenceServiceGenerateTokensResponse,
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
                  isThinking: false,
                  response: message.response,
                  thoughts: message.thoughts,
                });
              }

              if ("<think>" === token) {
                return Object.freeze({
                  errors: message.errors,
                  isEmpty: false,
                  isThinking: true,
                  response: message.response,
                  thoughts: message.thoughts,
                });
              }

              if ("</think>" === token) {
                return Object.freeze({
                  errors: message.errors,
                  isEmpty: false,
                  isThinking: false,
                  response: message.response,
                  thoughts: message.thoughts,
                });
              }

              if (message.isThinking) {
                return Object.freeze({
                  errors: message.errors,
                  isEmpty: false,
                  isThinking: true,
                  response: message.response,
                  thoughts: `${message.thoughts}${token}`,
                });
              }

              return Object.freeze({
                errors: message.errors,
                isEmpty: false,
                isThinking: false,
                response: `${message.response}${token}`,
                thoughts: message.thoughts,
              });
            }, defaultMessage),
          )
          .subscribe(setMessage);

        return function () {
          subscription.unsubscribe();
        };
      },
      [inferenceSocketClient, setMessage, submittedPrompt, version],
    );

    if (message.isEmpty) {
      if (submittedPrompt) {
        return (
          <ConversationMessage
            author="AI"
            errors={message.errors}
            isThinking={true}
            thoughts={message.thoughts}
          >
            <Markdown>{message.response}</Markdown>
          </ConversationMessage>
        );
      }

      return;
    }

    return (
      <ConversationMessage
        author="AI"
        errors={message.errors}
        isThinking={message.isThinking}
        thoughts={message.thoughts}
      >
        <Markdown>{message.response}</Markdown>
      </ConversationMessage>
    );
  },
);
