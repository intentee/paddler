---
paths:
  - "paddler_messaging/**"
---

# Paddler Messaging Context

- `paddler_messaging` must contain only messaing protocol between `paddler_agent`, and `paddler_balancer`
- `paddler_messaging` must only be a thin messaging, and validation layer between `paddler_agent`, and `paddler_balancer`
- `paddler_messaging` is intended to be used in `paddler_client`, and must not pull heavy dependencies like `llama-cpp-bindings`
