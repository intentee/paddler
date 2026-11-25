---
paths:
  - "paddler_cluster_harness/**"
---

# Paddler Cluster Harness Context

- `paddler_cluster_harness` starts and drives Paddler clusters (balancers and agents, in-process or as separate processes) for the test packages of every crate
- `Cluster::start_in_process` starts a balancer and its agents in the test process; `paddler_cli_tests` starts them as separate processes through the same `Cluster`
- The `run_with_test_cluster` binary runs a command (the Python suites) against a cluster started from a `TestClusterPreset`

# OpenAI Compatibility Testing

- To stay objective, we must not implement our own OpenAI client, instead we need to use a vetted 3rd party (preferably official OpenAI API client)
