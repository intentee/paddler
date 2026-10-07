---
paths:
  - "paddler_service_thread/**"
---

# Paddler Service Thread Context

- `paddler_service_thread` runs a bundle of trzcina services on a dedicated thread until it is cancelled, and reports how that thread ended
- `ServiceThreadError` is its single error enum
- it knows nothing about the balancer or the agent; `paddler_balancer_runner` and `paddler_agent_runner` build on it
