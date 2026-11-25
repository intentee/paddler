---
paths:
  - "paddler_balancer_typesafe_service/**"
---

# Paddler Balancer TypeSafe Service Context

- `paddler_balancer_typesafe_service` serves the TypeSafe-compatible API (models, system one decisions) of the balancer
- It plugs `TypeSafeCompatibilityLayer` into the compatibility service framework of `paddler_balancer`; `paddler_bootstrap` starts it
- Translation between TypeSafe and Paddler messages belongs to `paddler_typesafe_translation`; this crate only serves HTTP and relays decisions
