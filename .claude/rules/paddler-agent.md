---
paths:
  - "paddler_agent/**"
---

# Paddler Agent Context

- only the agent crates (`paddler_agent` and the `paddler_agent_*` pipeline crates: `paddler_agent_runtime`, `paddler_agent_text_generation`, `paddler_agent_embeddings`, `paddler_agent_decision`, `paddler_agent_pointer_head`) can rely on `llama-cpp-bindings`; balancer-side crates never do
- `paddler_agent` connects to the balancer, reconciles the desired state, and runs the inference pipeline that serves it; each pipeline lives in its own `paddler_agent_*` crate
- no crate can depend directly on `paddler_agent` (besides `paddler_agent_runner`, and other test related crates), they need to use `paddler_messaging` instead

