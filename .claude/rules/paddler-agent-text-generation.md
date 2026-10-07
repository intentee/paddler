---
paths:
  - "paddler_agent_text_generation/**"
---

# Paddler Agent Text Generation Context

- `paddler_agent_text_generation` is the agent pipeline that continues conversations and raw prompts token by token: chat templates, multimodal prompts, sampling, grammars, and tool calls
- it builds on `paddler_agent_runtime` and never depends on another pipeline crate
- `TextGenerationError` is its single error enum
