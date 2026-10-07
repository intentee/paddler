---
paths:
  - "paddler_agent_pointer_head/**"
---

# Paddler Agent Pointer Head Context

- `paddler_agent_pointer_head` reads a kev pointer head from its GGUF file and turns hidden states into option probabilities
- it knows only the pointer-head GGUF contract that `paddler_kev_converter_python` writes; its tests read the committed `fixtures/*pointer_head*.gguf` files, so they double as the contract test
- it never depends on a model or a llama.cpp context; checking a pointer head against the model it serves belongs to the decision pipeline
- `PointerHeadError` is its single error enum
