---
paths:
  - "paddler_client_tests/**"
---

# Paddler Client Tests Context

- `paddler_client_tests` tests `paddler_client` against spawned Paddler balancers; it cannot live in `paddler_client/tests` because `paddler_cluster_harness` depends on `paddler_client`
