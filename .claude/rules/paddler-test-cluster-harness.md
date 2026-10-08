---
paths:
  - "paddler_test_cluster_harness/**"
---

# Paddler Test Cluster Harness Context

- `paddler_test_cluster_harness` provides common test harness to be used with `paddler_bootstrap` tests, `paddler_cli_tests`, `paddler_gui` tests, `paddler_gui_tests`, `paddler_opencode_tests`, and `paddler_tests`

# OpenAI Compatibility Testing

- To stay objective, we must not implement our own OpenAI client, instead we need to use a vetted 3rd party (preferably official OpenAI API client)
