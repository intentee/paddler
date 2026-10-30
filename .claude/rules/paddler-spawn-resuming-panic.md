---
paths:
  - "paddler_spawn_resuming_panic/**"
---

# Paddler Spawn Resuming Panic Context

- `paddler_spawn_resuming_panic` is a root Paddler crate, it must not depend on any other Paddler crate
- `paddler_spawn_resuming_panic` runs work on a spawned or blocking tokio task, and resumes the task's panic in the task that awaits it, so the work fails exactly as if it ran inline
