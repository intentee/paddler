---
paths:
  - "paddler_test_cluster_harness/**"
---

# Paddler Test Cluster Harness Context

- `paddler_test_cluster_harness` provides common test harness to be used with `paddler_balancer_runner` tests, `paddler_agent_runner` tests, `paddler_cli_tests`, `paddler_gui` tests, `paddler_gui_tests`, `paddler_opencode_tests`, and `paddler_tests`

# OpenAI Compatibility Testing

- To stay objective, we must not implement our own OpenAI client, instead we need to use a vetted 3rd party (preferably official OpenAI API client)

# TypeSafe Compatibility Testing

- TypeSafe publishes no Rust SDK, so the harness's `TypeSafeApiClient` is a thin `reqwest` client built on the balancer's `TypeSafeApiPath` and `TypeSafeHeader`
- the official `typesafe-sdk` is exercised by `paddler_typesafe_client_python_test`, which keeps the compatibility claim objective
- both layers expose the same harness surface: `RunningBalancer::compat_<vendor>_addr()`, `compat_<vendor>_base_url()`, and `Cluster::compat_<vendor>_health_client()`
