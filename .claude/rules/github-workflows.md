---
paths:
  - ".github/workflows/**/*"
---

# GitHub Workflows Standards

- Always use Makefile targets in the workflow to avoid code duplication (if they need to run something that is already present in a Makefile).
- Never add the tests that use LLMs to GitHub workflows, because the default GitHub worker does not have the capacity to run them.
- Only add unit tests to GitHub workflows.
- Keep GitHub workflows responsible for only a single concern. For example, run linter, and tests in parallel.
- Treat GitHub workflows as a coding project. Use composable actions, factor similar concerns into actions.
- Encapsulate functionalities in composable actions.
- Keep the workflows clean and purposeful.
