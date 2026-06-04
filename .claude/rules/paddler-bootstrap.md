---
paths:
  - "paddler_bootstrap/**"
---

# Paddler Bootstrap Context

- `paddler_bootstrap` is used both by `paddler_cli`, `paddler_tests`, and `paddler_gui`
- `paddler_bootstrap` combines both `paddler_agent`, and `paddler_balancer`, and provides a unified entry point
- `paddler_bootstrap` is the canonical way to start both core paddler services (balancer, and agent)
- `paddler_bootstrap` is the source of truth on how to start Paddler services
