---
paths:
  - "**/Makefile"
---

# Makefile Standards

- Keep variables at the top of the file. Always.
- Prefer real targets over phony targets. If something can be express as a real target, do that.
- If you see that a phony target can be expressed as a real target, you can suggest a fix.
- Keep real targets, phony targets grouped together. Keep targets alphabetically sorted within each group.
- Keep all the real targets above phony targets.
- Make sure each Makefile target has enough dependencies to be able to run from a clean state.
