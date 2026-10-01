---
paths:
  - "paddler_gui_tests/**"
---

# Paddler GUI Tests Context

- `paddler_gui_tests` contains headless tests of `paddler_gui` built on `iced_test`, so test-only libraries stay out of the production GUI crate
- `paddler_gui_tests` drives the real application: its views, its update transitions, the tasks it returns, and its subscriptions
- `paddler_gui_tests` must not depend on system fonts, window systems, or GPUs to decide whether a test passes
