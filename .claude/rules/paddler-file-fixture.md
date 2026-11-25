---
paths:
  - "paddler_file_fixture/**"
---

# Paddler File Fixture Context

- `paddler_file_fixture` is the single resolver of the repository-wide `fixtures/` directory; tests never build fixture paths themselves
- Image fixtures are `ImageFixture` constants that encode themselves as data URIs
