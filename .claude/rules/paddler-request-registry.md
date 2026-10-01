---
paths:
  - "paddler_request_registry/**"
---

# Paddler Request Registry Context

- `paddler_request_registry` is a root Paddler crate, it must not depend on any other Paddler crate
- `paddler_request_registry` keeps in-flight requests addressable by their request id, so `paddler_agent` and `paddler_balancer` share one registration, delivery, and deregistration mechanism
