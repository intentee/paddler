---
paths:
  - "paddler_state_database/**"
---

# Paddler State Database Context

- `paddler_state_database` persists the balancer desired state, in a file or in memory
- `paddler_state_database` depends only on `paddler_messaging` among the Paddler crates
- `paddler_state_database` requires absolute file paths and treats an unreadable or corrupt state file as an error instead of replacing it with defaults
