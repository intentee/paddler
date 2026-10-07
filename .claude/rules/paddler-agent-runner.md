---
paths:
  - "paddler_agent_runner/**"
---

# Paddler Agent Runner Context

- `paddler_agent_runner` is the canonical way to start the agent services; `paddler_cli`, `paddler_gui`, and `paddler_tests` all start the agent through it
- it has no failure modes of its own, so it reports `ServiceThreadError` from `paddler_service_thread` instead of declaring an error enum
- it never depends on `paddler_balancer_runner`
