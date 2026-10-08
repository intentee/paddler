---
paths:
  - "paddler_agent/**"
---

# Paddler Agent Context

- `paddler_agent` is the only crate that can rely on `llama-cpp-bindings`
- agent is the only crate responsible for instantiating llama.cpp back-end, and communicating with it
- no crate can depend directly on `paddler_agent` (besides `paddler_bootstrap`, and other test related crates), they need to use `paddler_messaging` instead

