---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---

# Rust Standards

- Do not inline import paths unless necessary. Prefer to use `use` statements in Rust files instead of inline paths to imported modules. The exception would be `error.rs` type modules that handle lib-level error structs.
- Always use explicit lifetime variable names (do not use `'a` and such, use descriptive names like `'message` or similar)
- Always use explicit generic parameter names (never use single letter names like `T` for generics, prefix all of them with `T`, however). For example, use `TMessage` instead of `T`, etc.
- Do not use `pub(crate)` in Rust; in case of doubt, just make things public.
- In Rust, never ignore errors with `Err(_)`; always make sure you are matching an expected error variant instead.
- Never use `.expect`, or `.unwrap`. In Rust, if a function can fail, use a matching Result (can be from the anyhow crate) instead. In case of doubt on this, ask. Allow `.expect` in mutex lock poison checks, or when integrating CPP libraries into Rust.
- Always make sure mutex locks are held for the shortest possible time.
- Always specify Rust dependencies in root Cargo.toml, then use workspace versions of packages in workspace members.
- In Rust, when implementing a `new` method in a struct, prefer to use a struct with a parameter list instead of multiple function arguments. It should be easier to maintain.
- Always check the project with Clippy.
- Always format the code with `cargo fmt`.
