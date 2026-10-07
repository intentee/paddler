---
paths:
  - "paddler_tests/**"
---

# Paddler Tests Context

- `paddler_tests` contains Paddler integration tests
- `paddler_tests` contain utilities related to the integration tests
- its `run_with_test_cluster` binary starts a test cluster from a named preset and runs a command with the cluster's URLs in the environment; the vendor SDK suites (`paddler_openai_client_python_test`, `paddler_typesafe_client_python_test`) run inside it, so cluster setup lives only in Rust
