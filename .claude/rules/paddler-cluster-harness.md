---
paths:
  - "paddler_cluster_harness/**"
---

# Paddler Cluster Harness Context

- `paddler_cluster_harness` starts and drives Paddler clusters (balancers and agents, in-process or as separate processes) for the test packages of every crate

# OpenAI Compatibility Testing

- To stay objective, we must not implement our own OpenAI client, instead we need to use a vetted 3rd party (preferably official OpenAI API client)
