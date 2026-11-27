---
paths:
  - "paddler_agent_model_capabilities/**"
---

# Paddler Agent Model Capabilities Context

- `paddler_agent_model_capabilities` establishes what a loaded model serves, once per serving setup: it resolves the chat templates, loads the multimodal projection, and detects the capabilities through llama-cpp-bindings
- The established capabilities are stored and only read afterwards: pipelines start from them, the agent status reports them, and requests are gated by them
- It encodes no model knowledge; every capability comes from llama-cpp-bindings
