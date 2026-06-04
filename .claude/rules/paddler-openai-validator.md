---
paths:
  - "paddler_openai_validator/**"
---

# Paddler OpenAI Validator Context

- `paddler_openai_validator` is intended to ONLY be used in test crates, unit tests, and such
- `paddler_openai_validator` is only intended to be used to validate Paddler's OpenAI compatibility endpoints
- `paddler_openai_validator` must NOT be used on runtime; it must ONLY be used in tests, unit tests, integration tests
- `paddler_openai_validator` must directly use vendored, official OpenAI schema to build its validation setup
- `paddler_openai_validator` must make the official OpenAI schema stricture, to make sure Paddler does not introduce extra fields to the requests
- `paddler_openai_validator` must make the official OpenAI schema stricture, to make sure Paddler does not accept unsupported fields
