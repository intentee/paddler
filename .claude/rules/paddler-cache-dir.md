---
paths:
  - "paddler_cache_dir/**"
---

# Paddler Cache Directory Context

- `paddler_cache_dir` is a root Paddler crate, it must not depend on any other Paddler crate, except dependency-free utility crates such as `paddler_sha256_hex`
- `paddler_cache_dir` manages Paddler's global cache directory, and all its nuances
- `paddler_cache_dir` resolves OS-related differences internally
- `paddler_cache_dir` must use cache directory patterns idiomatic to the specific operating system
