import React, { memo, useContext, useEffect, useMemo, useState } from "react";
import { scan } from "rxjs";

import { inferenceSocketClient } from "@intentee/paddler-client/inferenceSocketClient";
import { type ConversationMessage as ConversationMessageType } from "@intentee/paddler-client/schemas/ConversationMessage";
import { type ConversationMessageContentPart } from "@intentee/paddler-client/schemas/ConversationMessageContentPart";
import { type InferenceServiceGenerateTokensResponse } from "@intentee/paddler-client/schemas/InferenceServiceGenerateTokensResponse";
import { PromptContext } from "../contexts/PromptContext";
import { PromptMediaContext } from "../contexts/PromptMediaContext";
import { PromptSystemMessageContext } from "../contexts/PromptSystemMessageContext";
import { PromptThinkingContext } from "../contexts/PromptThinkingContext";
import { type PromptMedia } from "../PromptMedia";
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

const systemMessage: ConversationMessageType = Object.freeze({
  Message: {
    role: "system",
    content:
      "You are a helpful assistant. Give engaging, short, precise answers. Be friendly, supportive, use emojis.",
  },
});

const assistantGreeting: ConversationMessageType = Object.freeze({
  Message: {
    role: "assistant",
    content: "Hello! How can I help you today?",
  },
});

function mediaContentPart(media: PromptMedia): ConversationMessageContentPart {
  if ("audio" === media.kind) {
    return { type: "input_audio", input_audio: media.audio };
  }

  return { type: "image_url", image_url: { url: media.dataUri } };
}

function buildUserMessage(
  submittedPrompt: string,
  submittedMedia: null | PromptMedia,
): ConversationMessageType {
  if (submittedMedia) {
    return {
      Message: {
        role: "user",
        content: [
          mediaContentPart(submittedMedia),
          {
            type: "text",
            text: submittedPrompt,
          },
        ],
      },
    };
  }

  return {
    Message: {
      role: "user",
      content: submittedPrompt,
    },
  };
}

export const ConversationMessagePromptGeneratedTokens = memo(
  function ConversationMessagePromptGeneratedTokens({
    webSocket,
  }: {
    webSocket: WebSocket;
  }) {
    const { submittedPrompt, version } = useContext(PromptContext);
    const { submittedMedia } = useContext(PromptMediaContext);
    const { submittedIsThinkingEnabled } = useContext(PromptThinkingContext);
    const { submittedIsSystemMessageIncluded } = useContext(
      PromptSystemMessageContext,
    );
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
              ...(submittedIsSystemMessageIncluded ? [systemMessage] : []),
              assistantGreeting,
              buildUserMessage(submittedPrompt, submittedMedia),
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
        submittedMedia,
        submittedIsSystemMessageIncluded,
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
