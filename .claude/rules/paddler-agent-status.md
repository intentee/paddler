---
paths:
  - "paddler_agent_status/**"
---

# Paddler Agent Status Context

- `paddler_agent_status` depends only on `paddler_messaging` among the Paddler crates
- `paddler_agent_status` aggregates what an agent reports about itself: its slots, its model, its download progress, and its issues
- `paddler_agent_status` is shared by `paddler_agent`, `paddler_model_source`, and `paddler_bootstrap`, so none of them needs to depend on another to report agent status
