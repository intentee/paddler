import { nanoid } from "nanoid";
import { filter, fromEvent, map, takeWhile, type Observable } from "rxjs";

import {
  InferenceServiceGenerateTokensResponseSchema,
  type InferenceServiceGenerateTokensResponse,
} from "./schemas/InferenceServiceGenerateTokensResponse";
import {
  InferenceNotificationSchema,
  type InferenceNotification,
} from "./schemas/InferenceNotification";
import type { ConversationMessage } from "./schemas/ConversationMessage";

export interface InferenceSocketClient {
  clusterPromptingMode$: Observable<InferenceNotification>;
  continueConversation(params: {
    enableThinking: boolean;
    messages: ConversationMessage[];
  }): Observable<InferenceServiceGenerateTokensResponse>;
}

function isNotificationFrame(parsedFrame: unknown): boolean {
  return (
    "object" === typeof parsedFrame &&
    null !== parsedFrame &&
    "Notification" in parsedFrame
  );
}

export function inferenceSocketClient({
  webSocket,
}: {
  webSocket: WebSocket;
}): InferenceSocketClient {
  const parsedFrames$: Observable<unknown> = fromEvent<MessageEvent>(
    webSocket,
    "message",
  ).pipe(
    map(function (event): unknown {
      return event.data;
    }),
    filter(function (eventData) {
      return "string" === typeof eventData;
    }),
    map(function (serializedFrame: string): unknown {
      return JSON.parse(serializedFrame);
    }),
  );

  const clusterPromptingMode$: Observable<InferenceNotification> =
    parsedFrames$.pipe(
      filter(isNotificationFrame),
      map(function (parsedFrame: unknown): InferenceNotification {
        return InferenceNotificationSchema.parse(parsedFrame).Notification;
      }),
    );

  function continueConversation({
    enableThinking,
    messages,
  }: {
    enableThinking: boolean;
    messages: ConversationMessage[];
  }): Observable<InferenceServiceGenerateTokensResponse> {
    const requestId = nanoid();
    const tokenStream = parsedFrames$.pipe(
      filter(function (parsedFrame) {
        return !isNotificationFrame(parsedFrame);
      }),
      map(function (parsedFrame: unknown) {
        try {
          return InferenceServiceGenerateTokensResponseSchema.parse(
            parsedFrame,
          );
        } catch (error: unknown) {
          console.error(
            "Failed to parse token response frame:",
            parsedFrame,
            error,
          );

          throw error;
        }
      }),
      filter(function ({ request_id }) {
        return request_id === requestId;
      }),
      takeWhile(function ({ done }) {
        return !done;
      }, true),
    );

    webSocket.send(
      JSON.stringify({
        Request: {
          id: requestId,
          request: {
            ContinueFromConversationHistory: {
              add_generation_prompt: true,
              conversation_history: messages,
              enable_thinking: enableThinking,
              max_tokens: 32768,
            },
          },
        },
      }),
    );

    return tokenStream;
  }

  return Object.freeze({
    clusterPromptingMode$,
    continueConversation,
  });
}
