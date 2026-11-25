---
paths:
  - "paddler_balancer_state_database/**"
---

# Paddler Balancer State Database Context

- `paddler_balancer_state_database` persists the balancer desired state, in a file or in memory
- `paddler_balancer_state_database` depends only on `paddler_messaging`, `paddler_inference_parameters` and the root `paddler_spawn_resuming_panic` among the Paddler crates
- `paddler_balancer_state_database` reads every schema version it ever wrote and migrates it to the current one
- `paddler_balancer_state_database` requires absolute file paths and treats an unreadable or corrupt state file as an error instead of replacing it with defaults
