---
paths:
  - "paddler_balancer_openai_service_tests/**"
---

# Paddler Balancer OpenAI Service Tests Context

- `paddler_balancer_openai_service_tests` tests the OpenAI-compatible API of the balancer against spawned Paddler clusters, through the official OpenAI client and the real OpenCode CLI
- The OpenCode tests reuse `paddler_cluster_harness` to start Paddler; they never reimplement cluster startup.
- The OpenCode binary path is always provided by the user via the `PADDLER_OPENCODE_BINARY` environment variable and is never guessed.
- The OpenCode end-to-end tests are gated behind both the `tests_that_use_opencode` and `tests_that_use_llms` features; when they run without `PADDLER_OPENCODE_BINARY`, they fail rather than skip.
- The OpenCode model is always `ministral_3_3b_reasoning`, since it is the smallest model that drives OpenCode 2's Code Mode `execute` tool reliably; only the OpenCode binary path is configurable.
