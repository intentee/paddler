import { deepStrictEqual } from "node:assert/strict";
import { test } from "node:test";
import { lastValueFrom, toArray } from "rxjs";

import { InferenceServiceGenerateTokensResponseSchema } from "../../src/schemas/InferenceServiceGenerateTokensResponse";
import { streamHttpNdjson } from "../../src/streamHttpNdjson";
import { qwen3DesiredState } from "../qwen3DesiredState";
import { withPaddlerCluster } from "../withPaddlerCluster";

const BAD_REQUEST = 400;

test("streamHttpNdjson delivers an invalid tool schema rejection", async function () {
  const responses = await withPaddlerCluster(
    {
      agents: [{ name: "qwen3-agent", slots: 1 }],
      configureDesiredState: qwen3DesiredState,
    },
    function ({ inference }) {
      return lastValueFrom(
        streamHttpNdjson({
          body: {
            add_generation_prompt: true,
            conversation_history: [
              { content: "What is the weather in Paris?", role: "user" },
            ],
            enable_thinking: false,
            max_tokens: 64,
            parse_tool_calls: true,
            tools: [
              {
                function: {
                  description: "Get the current weather for a location",
                  name: "get_weather",
                  parameters: {
                    properties: { location: { type: 123 } },
                    type: "object",
                  },
                },
                type: "function",
              },
            ],
          },
          schema: InferenceServiceGenerateTokensResponseSchema,
          signal: new AbortController().signal,
          url: `http://${inference}/api/v1/continue_from_conversation_history`,
        }).pipe(toArray()),
      );
    },
  );

  deepStrictEqual(
    responses.map(function ({ done, error }) {
      return { done, errorCode: error?.code };
    }),
    [{ done: true, errorCode: BAD_REQUEST }],
  );
});
