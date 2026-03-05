import { useEffect, useState } from "react";

import { InferenceServiceGenerateTokensResponseSchema } from "../schemas/InferenceServiceGenerateTokensResponse";

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

      fetch(`//${inferenceAddr}/api/v1/continue_from_conversation_history`, {
        body: JSON.stringify({
          add_generation_prompt: true,
          conversation_history: {
            messages: [
              { role: "assistant", content: systemPrompt },
              { role: "user", content: prompt },
            ],
          },
          enable_thinking: false,
          max_tokens: 300,
        }),
        headers: {
          "Content-Type": "application/json",
        },
        method: "POST",
        signal: abortController.signal,
      })
        .then(function ({ body }) {
          if (!body) {
            throw new Error("No response body");
          }

          return body.getReader();
        })
        .then(async function (reader) {
          const decoder = new TextDecoder();

          // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
          while (true) {
            const { done, value } = await reader.read();

            if (done || abortController.signal.aborted) {
              return;
            }

            const chunk = decoder.decode(value, {
              stream: true,
            });

            const lines = chunk.split("\n").filter(function (line) {
              return line.trim();
            });

            for (const line of lines) {
              try {
                const message = JSON.parse(line);
                const validatedMessage =
                  InferenceServiceGenerateTokensResponseSchema.parse(message);

                if (validatedMessage.done) {
                  return;
                }

                setMessage(function (prevMessage) {
                  return `${prevMessage}${validatedMessage.token}`;
                });
              } catch (err) {
                console.error("Error:", err);
              }
            }
          }
        })
        .catch(function (error: unknown) {
          console.error("Error during fetch:", error);
        });

      return function () {
        abortController.abort();
      };
    },
    [inferenceAddr, prompt, systemPrompt],
  );

  return {
    message,
  };
}
