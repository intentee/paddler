---
paths:
  - "paddler_balancer_runner/**"
---

# Paddler Balancer Runner Context

- `paddler_balancer_runner` is the canonical way to start the balancer services; `paddler_cli`, `paddler_gui`, and `paddler_tests` all start the balancer through it
- `BalancerRunnerError` is its single error enum
- it never depends on `paddler_agent_runner` (its tests may, to exercise a real agent connecting)
