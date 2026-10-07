---
paths:
  - "paddler_kev_converter_python/**"
  - "paddler_llama_cpp_converter_python/**"
---

# Paddler Kev Converter Context

- `paddler_kev_converter_python` turns a kev decision checkpoint into the files Paddler serves: a `Qwen3_5ForCausalLM` Hugging Face directory with the LoRA merged in fp32 and the base's own tokenizer files, plus the pointer-head GGUF
- it also writes the committed reference fixtures (synthetic pointer heads, Hugging Face tokenizer ids, TypeSafe renderings and answers, and the Kev-0.8B fp32 parity reference) through the `fixtures/*` Makefile targets; regenerate them with make, never by hand
- it reuses kev itself (`kev.checkpoint`, `kev.model`, `kev.api`), pinned to an exact commit; `stubs/kev` declares the typed surface of the kev APIs it calls and must follow the pinned kev
- it refuses checkpoints it cannot convert exactly (full weights, non-fp32 backbones, option isolation, trained token embeddings, bases other than `qwen3_5_text`) with one exception class per refusal
- `paddler_llama_cpp_converter_python` only pins the runtime of llama.cpp's `convert_hf_to_gguf.py`; the script itself comes from the llama.cpp sources the `llama-cpp-bindings-sys` crate ships, so the GGUF always matches the llama.cpp Paddler builds
- the two converters need separate environments: kev needs transformers 5, llama.cpp's converter pins transformers 4
- the pointer-head GGUF contract: architecture `pointer_head`; F32 tensors `pointer_head.{query,key}.{weight,bias}` (the hidden size comes from their shapes); `pointer_head.temperature`, and `pointer_head.delimiter.{state,question,option_start,option_end,decide}` as token strings
