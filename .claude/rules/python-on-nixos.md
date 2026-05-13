---
paths:
  - "paddler_client_python/**/*"
---

# Running Python tooling on NixOS

To run any Python tool that may have ELF / dynamic-linker issues on NixOS — `ruff`, `mypy`, `pyright`, `pytest`, anything installed from a pip wheel with native 
bits — first enter `paddler_client_python/shell.nix`, then drive everything through `poetry` from inside that shell.

**Why:** 
pip wheels like `ruff` ship a generic-linux binary, which NixOS does not provide. 
Running them directly fails with `Could not start dynamically linked executable: ... NixOS cannot run dynamically linked executables intended for generic linux environments`. 
`shell.nix` provides the Nix-built loader / replacement tools that make those binaries (or their Nix equivalents) actually launch. 
`poetry` is just the dispatcher you use *inside* that prepared shell — never the entry point on its own.

**How to apply:**
- Never invoke `ruff`, `poetry run ...`, `python`, `pytest`, etc. from outside `nix-shell`. If a command starts with one of those, it must be inside `nix-shell --run "..."`.
- If `paddler_client_python/shell.nix` is missing, stop and ask. Adding a `shell.nix` is the fix; running tooling unwrapped is not.
