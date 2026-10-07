---
paths:
  - "paddler_agent_embeddings/**"
---

# Paddler Agent Embeddings Context

- `paddler_agent_embeddings` is the agent pipeline that turns batches of documents into embeddings
- it builds on `paddler_agent_runtime` and never depends on another pipeline crate
- `EmbeddingError` is its single error enum
