---
paths:
  - "paddler_tcp_buffer_limits/**"
---

# Paddler TCP Buffer Limits Context

- `paddler_tcp_buffer_limits` is a test fixture crate, it must not depend on any other Paddler crate
- `paddler_tcp_buffer_limits` reads the kernel TCP buffer limits, so tests can size the data that outlasts both socket buffers of a loopback connection instead of guessing
