---
paths:
  - "paddler_cache_dir/**"
---

# Paddler Cache Directory Context

- `paddler_cache_dir` is a root Paddler crate, it must not depend on any other Paddler crate
- `paddler_cache_dir` manages Paddler's global cache directory, and all its nuances
- `paddler_cache_dir` resolves the cache directory from `PADDLER_CACHE_DIR`, then `$XDG_CACHE_HOME/paddler`, then `$HOME/.cache/paddler`
- Paddler supports Linux and macOS only, so the cache directory follows the XDG convention on both
