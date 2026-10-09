import { nanoid } from "nanoid";
import { filter, fromEvent, map, takeWhile, type Observable } from "rxjs";

import type { ConversationMessage } from "./schemas/ConversationMessage";
import type { InferenceMode } from "./schemas/InferenceMode";
import { InferenceNotificationSchema } from "./schemas/InferenceNotification";
import {
  InferenceServiceGenerateTokensResponseSchema,
  type InferenceServiceGenerateTokensResponse,
} from "./schemas/InferenceServiceGenerateTokensResponse";

export interface InferenceSocketClient {
  clusterInferenceMode$: Observable<InferenceMode>;
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

  const clusterInferenceMode$: Observable<InferenceMode> = parsedFrames$.pipe(
    filter(isNotificationFrame),
    map(function (parsedFrame: unknown): InferenceMode {
      return InferenceNotificationSchema.parse(parsedFrame).Notification
        .ClusterInferenceMode;
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
        return InferenceServiceGenerateTokensResponseSchema.parse(parsedFrame);
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
    clusterInferenceMode$,
    continueConversation,
  });
}
