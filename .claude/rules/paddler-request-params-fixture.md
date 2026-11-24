---
paths:
  - "paddler_request_params_fixture/**"
---

# Paddler Request Params Fixture Context

- `paddler_request_params_fixture` holds the inference request parameters (conversations, decisions, embedding documents, prompt texts) that more than one test package sends
- It holds data only; anything that drives a pipeline or a cluster belongs to a harness
