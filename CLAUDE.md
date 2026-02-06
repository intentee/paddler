# Project Context

When working with this codebase, prioritize readability over cleverness. Ask clarifying questions before making architectural changes.

Keep it simple, be opinionated, follow best practices. Avoid using configurable parameters.

Keep the code beautiful. Always optimize the code for a great developer experience.

## Coding Standards

- Do not inline import paths unless necessary. Prefer to use `use` statements in Rust files instead of inline paths to imported modules. The exception would be `error.rs` type modules that handle lib-level error structs.
- Keep at most a single public struct per Rust module.
- Keep at most a single public function per Rust module (multiple public struct methods are OK).
- Keep module names elegant and clearly readable. The name of the module, or any file, should be enough to determine its contents unambiguously.
- Keep modules structure as flat as possible, avoid logical grouping of modules, instead keep the naming consistent.
- Keep standalone, private functions and structs above the public struct or function that is exported.
- Group the modules by name prefix. For example, `client_foo`, `client_bar`, etc., wherever it makes sense to do so.
- Decide to group the modules based on software architecture, messaging hierarchy, or inheritance. Do not group modules just for the sake of it.
- Maintain a tree-like structure of modules, avoid circular dependencies at all costs. Extract common functions or structs into separate modules, or separate subprojects in the workspace.
- Name files the same way as the struct or function they contain.
- Be explicit, do not use general import statements that involve "*", prefer to import everything explicitly.
- Do not use copy-pasted or copied code in any capacity. If you have issues extracting something into a module, discuss the steps first.
- Keeping slightly different message types, or other kinds of structs that are only slightly different, because of the context they are used in, is fine.
- Each function or method should do just a single thing. The single responsibility principle is really important.
- Always use explicit lifetime variable names (do not use `'a` and such, use descriptive names like `'message` or similar)
- Always use explicit generic parameter names (never use single letter names like `T` for generics, prefix all of them with `T`, however). For example, use `TMessage` instead of `T`, etc.
- Always use descriptive and explicit variable names, even in anonymous functions. Never use single-letter variable names.
- Instead of writing comments that explain what the code does, make the code self-documenting.
- Do not use `pub(crate)` in Rust; in case of doubt, just make things public.
- Add an empty line before return statements that end the function or a method.
- Add an empty line between loops and preceding statements from the same scope.
- Handle all the errors; never ignore them. Make sure the application does not panic.
- In Rust, never ignore errors with `Err(_)`; always make sure you are matching an expected error variant instead.
- Never use `.expect`, or `.unwrap`. In Rust, if a function can fail, use a matching Result (can be from the anyhow crate) instead. In case of doubt on this, ask. Allow `.expect` in mutex lock poison checks, or when integrating CPP libraries into Rust.
- Use object-oriented style and composition. Avoid functions that take a struct as a parameter; move it to the struct implementation instead.
- Always make sure mutex locks are held for the shortest possible time.
- Always specify Rust dependencies in root Cargo.toml, then use workspace versions of packages in workspace members.
- Avoid unnecessary abstractions.
- Before using vendor crates or modules, make sure they are well-maintained, secure, and documented.

## Quality Control

- Always check the project with Clippy.
- Always format the code with `cargo fmt`.
- Always test the code, make sure tests work after the changes.


## Committing Changes

- Always keep the commit messages short, human readable, descriptive. Keep commit messages as one-liners.
