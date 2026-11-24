---
paths:
  - "paddler_agent_pipeline_harness/**"
---

# Paddler Agent Pipeline Harness Context

- `paddler_agent_pipeline_harness` loads models into real, in-process agent pipelines (runtime context, loaded models, pipeline arbiters, agent requests) for the test packages of the agent crates
- It drives production pipelines only; it never reimplements or fakes Paddler behaviour
