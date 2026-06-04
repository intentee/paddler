---
paths:
  - "paddler_download_manager/**"
---

# Paddler Download Manager Context

- `paddler_download_manager` is a root Paddler crate, it must not depend on any other Paddler crate
- `paddler_download_manager` is responsible for downloading GGUF models from HTTP URLs
- `paddler_download_manager` must be resilient, it must support resumes, handle cache corruptions
- `paddler_download_manager` must not do retries, because it is intended to be used by `paddler_agent`, and `paddler_agent` already has a built-in retry mechanism
- `paddler_download_manager` must focus only on Paddler internal use-cases related to downloading models
