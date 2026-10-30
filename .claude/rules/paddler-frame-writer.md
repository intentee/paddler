---
paths:
  - "paddler_frame_writer/**"
---

# Paddler Frame Writer Context

- `paddler_frame_writer` is a root Paddler crate, it must not depend on any other Paddler crate
- `paddler_frame_writer` writes queued frames to a connection's sink until the queue ends, a send fails, or the connection closes, so `paddler_agent` and `paddler_balancer` share one way to abandon the writes their peer stopped reading
