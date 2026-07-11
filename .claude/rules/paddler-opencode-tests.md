---
paths:
  - "paddler_opencode_tests/**"
---

# Paddler OpenCode Tests Context

- `paddler_opencode_tests` drives the real OpenCode CLI against a live Paddler cluster to guard OpenAI-compatibility regressions 
- `paddler_opencode_tests` must reuse the cluster harness from `paddler_tests` / `paddler_test_cluster_harness` to start Paddler; it must not reimplement cluster startup.
- The OpenCode binary path is always provided by the user via the `PADDLER_OPENCODE_BINARY` environment variable and is never guessed.
- The end-to-end tests are gated behind both the `tests_that_use_opencode` and `tests_that_use_llms` features; when they run without `PADDLER_OPENCODE_BINARY`, they fail rather than skip.
- The model is always `qwen3_5_0_8b` via the shared harness; only the OpenCode binary path is configurable.
