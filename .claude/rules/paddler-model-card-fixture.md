---
paths:
  - "paddler_model_card_fixture/**"
---

# Paddler Model Card Fixture Context

- `paddler_model_card_fixture` pins every Hugging Face model (repository, file, revision) that Paddler tests serve; it is the single source of truth for test models
- Each model card lives in its own module named after the model; `every_model_card` lists them all so `make target/test-model-cards.stamp` downloads and verifies every one
- Tests must use the smallest model that has the capability under test
