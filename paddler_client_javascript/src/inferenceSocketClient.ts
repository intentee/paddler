import { nanoid } from "nanoid";
import { filter, fromEvent, map, takeWhile, type Observable } from "rxjs";

import type { ConversationMessage } from "./schemas/ConversationMessage";
import {
  InferenceServiceGenerateTokensResponseSchema,
  type InferenceServiceGenerateTokensResponse,
} from "./schemas/InferenceServiceGenerateTokensResponse";

export interface InferenceSocketClient {
  continueConversation(params: {
    enableThinking: boolean;
    messages: ConversationMessage[];
  }): Observable<InferenceServiceGenerateTokensResponse>;
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

  function continueConversation({
    enableThinking,
    messages,
  }: {
    enableThinking: boolean;
    messages: ConversationMessage[];
  }): Observable<InferenceServiceGenerateTokensResponse> {
    const requestId = nanoid();
    const tokenStream = parsedFrames$.pipe(
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
    continueConversation,
  });
}
