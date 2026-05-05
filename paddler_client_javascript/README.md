# @intentee/paddler-client

JavaScript/TypeScript client for the [Paddler](https://github.com/intentee/paddler) LLM load balancer.

## Install

```sh
npm install @intentee/paddler-client rxjs zod
```

`rxjs` and `zod` are peer dependencies.

## Quick start

### WebSocket inference (multiplexed, request-id-keyed)

```ts
import { inferenceSocketClient } from "@intentee/paddler-client";

const webSocket = new WebSocket("ws://localhost:8061/api/v1/inference_socket");

const { continueConversation } = inferenceSocketClient({ webSocket });

continueConversation({
  enableThinking: true,
  messages: [
    { role: "system", content: "You are a helpful assistant." },
    { role: "user", content: "Hello" },
  ],
}).subscribe((chunk) => {
  if (chunk.error) {
    console.error(chunk.error);
    return;
  }
  if (chunk.done) {
    console.log("done", chunk.summary);
    return;
  }
  if (chunk.token !== null) {
    process.stdout.write(chunk.token);
  }
});
```

### HTTP NDJSON streaming

```ts
import { streamHttpNdjson, InferenceServiceGenerateTokensResponseSchema } from "@intentee/paddler-client";

const controller = new AbortController();

streamHttpNdjson({
  url: new URL("http://localhost:8061/api/v1/continue_from_conversation_history"),
  body: { add_generation_prompt: true, conversation_history: [...], max_tokens: 200 },
  signal: controller.signal,
  schema: InferenceServiceGenerateTokensResponseSchema,
}).subscribe(/* ... */);
```

### SSE management stream

```ts
import { streamEventSource, AgentsResponseSchema, matchEventSourceUpdateState } from "@intentee/paddler-client";

streamEventSource({
  url: new URL("http://localhost:8062/api/v1/agents/stream"),
  schema: AgentsResponseSchema,
}).subscribe((state) => {
  matchEventSourceUpdateState(state, {
    initial: () => console.log("connecting"),
    connected: () => console.log("connected"),
    dataSnapshot: ({ data }) => console.log("agents", data.agents.length),
    connectionError: () => console.error("connection lost"),
    deserializationError: () => console.error("invalid payload"),
  });
});
```

## Coverage

- Transport: WebSocket (multiplexed), HTTP NDJSON, HTTP JSON, Server-Sent Events
- Schemas: every Paddler wire-format type (validated via zod)
- State machines + exhaustive matchers for connection/stream/fetch states
- Specialized error types per failure mode

## License

Apache-2.0
