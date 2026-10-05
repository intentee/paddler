---
paths:
  - "paddler_model_source/**"
---

# Paddler Model Source Context

- `paddler_model_source` resolves the model an agent is asked to run (a Hugging Face reference, a URL, or a local path) into a local file
- `paddler_model_source` depends on `paddler_agent_status`, `paddler_cache_dir`, `paddler_download_manager`, and `paddler_messaging`, and must not depend on `paddler_agent`
- `paddler_model_source` reports download progress and model issues through `paddler_agent_status`
