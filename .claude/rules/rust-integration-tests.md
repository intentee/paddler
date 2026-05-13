---
paths:
  - "**/tests/**/*.rs"
---

# Rust Integration Tests Standards

- Each test needs to be named after what functionality, or issue it actually tests.
- Each test file needs to be named after what functionality, or issue it actually tests.
- Each test represents a specific scenario that the core project needs to support, or represent an uncovered issue.
- If you uncover a new issue while testing, create yet another targeted test that covers that.
- Every test muse use production code. Never recreate the original code to test something conceptually. Always use production code.
- They must be single-purpose.
- It must be clear what is being tested in the test file by just reading the filename.

