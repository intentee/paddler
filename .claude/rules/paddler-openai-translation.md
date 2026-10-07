---
paths:
  - "paddler_openai_translation/**"
---

# Paddler OpenAI Translation Context

- `paddler_openai_translation` translates OpenAI Chat Completions (`POST /v1/chat/completions`) and Responses (`POST /v1/responses`) requests into Paddler conversation requests, and generated tokens back into OpenAI completions, chunks, and stream events
- `ChatCompletionRequest::translate` and `ResponsesRequest::translate` produce a translated request whose delivery is either buffered (a header that `CompletesGeneration` turns into the whole response) or streamed (`ChatCompletionStream`, `ResponsesStream`)
- `GenerationEvent::from(GeneratedTokenResult)` is the single classification of generated tokens into output, the finish, and failures with their `GenerationFailureCause`
- a chat completion's `id` is the balancer's request id, so client-visible ids correlate with balancer logs
- it never depends on actix or the balancer; the OpenAI HTTP service builds on it
- `OpenAITranslationError` is its single error enum
