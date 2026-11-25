---
paths:
  - "paddler_refused_syscalls/**"
---

# Paddler Refused Syscalls Context

- `paddler_refused_syscalls` installs a seccomp filter that refuses chosen system calls of the test process, so tests can inject operating system failures into real Paddler code
- It is Linux-only; the filter applies to every thread of the process, so its tests rely on nextest running each test in its own process
