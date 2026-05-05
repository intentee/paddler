import { useEffect, useState } from "react";

import { InferenceServiceGenerateTokensResponseSchema } from "@intentee/paddler-client/schemas/InferenceServiceGenerateTokensResponse";
import { streamHttpNdjson } from "@intentee/paddler-client/streamHttpNdjson";

export function usePrompt({
  inferenceAddr,
  prompt,
  systemPrompt,
}: {
  inferenceAddr: string;
  prompt: string;
  systemPrompt: string;
}) {
  const [message, setMessage] = useState<string>("");

  useEffect(
    function () {
      const abortController = new AbortController();

      setMessage("");

      const subscription = streamHttpNdjson({
        url: `//${inferenceAddr}/api/v1/continue_from_conversation_history`,
        body: {
          add_generation_prompt: true,
          conversation_history: [
            { role: "assistant", content: systemPrompt },
            { role: "user", content: prompt },
          ],
          enable_thinking: false,
          max_tokens: 300,
        },
        signal: abortController.signal,
        schema: InferenceServiceGenerateTokensResponseSchema,
      }).subscribe({
        next(validatedMessage) {
          if (validatedMessage.done) {
            return;
          }

          if (null === validatedMessage.token) {
            return;
          }

          if ("content" !== validatedMessage.tokenKind) {
            return;
          }

          setMessage(function (prevMessage) {
            return `${prevMessage}${validatedMessage.token}`;
          });
        },
        error(error: unknown) {
          console.error("Error during fetch:", error);
        },
      });

      return function () {
        subscription.unsubscribe();
        abortController.abort();
      };
    },
    [inferenceAddr, prompt, systemPrompt],
  );

  return {
    message,
  };
}
