---
paths:
  - ".github/**/*"
---

# GitHub Workflows Standards

- Always use Makefile targets in the workflow to avoid code duplication.
- Never add the tests that use LLMs to GitHub workflows, because the default GitHub worker does not have the capacity to run them.
- Only add unit tests to GitHub workflows.
- Keep GitHub workflows responsible for only a single concern. For example, run linter, and tests in parallel.
