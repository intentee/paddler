---
paths:
  - "paddler_balancer/**"
---

# Paddler Balancer Context

- `paddler_balancer` crate is responsible for starting inference, and management servers
- Paddler Agents connect to the balancer in order to handle the requests that the balancer dispatches
- `paddler_balancer` provides the compatibility service framework (`compatibility` module) that vendor-compatible API crates plug their layers into; it never implements a vendor API itself
