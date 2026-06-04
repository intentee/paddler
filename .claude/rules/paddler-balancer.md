---
paths:
  - "paddler_balancer/**"
---

# Paddler Balancer Context

- `paddler_balancer` crate is responsible for starting inference, and management servers
- Paddler Agents connect to the balancer in order to handle the requests that the balancer dispatches
- `paddler_balancer` provides compatibility services that expose vendor-compatible APIs (for example OpenAI compatibility server)
