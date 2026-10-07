---
paths:
  - "paddler_agent_runtime/**"
---

# Paddler Agent Runtime Context

- `paddler_agent_runtime` holds what every agent inference pipeline shares: loading a model into llama.cpp, creating and warming up its contexts, running a pipeline's scheduler on its own thread, and the request every pipeline receives
- `paddler_agent_runtime` knows nothing about any particular pipeline; pipelines plug in through `PreparesSchedulerCommand` and `SchedulerCommand`
- `paddler_agent_runtime` depends only on `paddler_agent_status`, `paddler_inference_parameters`, and `paddler_messaging` among the Paddler crates
- `AgentRuntimeError` is its single error enum; pipeline crates compose it with `#[from]`
