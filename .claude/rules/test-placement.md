---
paths:
  - "**/*_tests/**"
  - "**/*_fixture/**"
  - "**/*_harness/**"
  - "**/tests/**"
---

# Test Placement

- A test lives with the crate whose code makes the decision its assertion checks: in that crate's `tests/` directory, or in its `*_tests` package when the test needs helpers that the crate cannot dev-depend on without a dependency cycle.
- A `*_tests` package holds the tests of exactly one crate, plus its own test library of helpers only those tests use.
- A helper used by more than one test package belongs to a shared test crate named after what it is: `*_fixture` for fixed test data or environment, `*_harness` for code that drives a running system under test.
- Tests that need two or more model-loading agents, process signals, or exit codes run the `paddler` executable as separate processes in `paddler_cli_tests`, because llama.cpp allows one backend per process.
- Tests drive real, spawned Paddler instances; only failure-injection peers (untrusted sockets, stallable relays, refused syscalls, llama.cpp fault injection) stand in for a counterpart.
