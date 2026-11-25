---
paths:
  - "paddler_balancer_openai_service/**"
---

# Paddler Balancer OpenAI Service Context

- `paddler_balancer_openai_service` serves the OpenAI-compatible API (chat completions, responses) of the balancer
- It plugs `OpenAICompatibilityLayer` into the compatibility service framework of `paddler_balancer`; `paddler_bootstrap` starts it
- Translation between OpenAI and Paddler messages belongs to `paddler_openai_translation`; this crate only serves HTTP and relays generations
