---
paths:
  - "paddler_agent_status/**"
---

# Paddler Agent Status Context

- `paddler_agent_status` depends only on `paddler_messaging` among the Paddler crates
- `paddler_agent_status` aggregates what an agent reports about itself: its slots, its model, its download progress, and its issues
- `paddler_agent_status` is shared by `paddler_agent`, the `paddler_agent_*` pipeline crates, `paddler_model_source`, and `paddler_agent_runner`, so none of them needs to depend on another to report agent status
- `SlotGuard` holds one processing slot for as long as a request is in flight
