---
paths:
  - "paddler_agent_decision/**"
---

# Paddler Agent Decision Context

- `paddler_agent_decision` is the agent pipeline that answers multiple-choice questions about a state document with a kev decision model: it never samples or generates text
- it lays out tokens the way kev does (`<state>` then, per question, `<question> instructions (<option> text </option>)… <decide>`), reads the post-norm hidden states of every `</option>` and `<decide>` token, and turns them into probabilities with `paddler_agent_pointer_head`
- every decode carries exactly one sequence: hybrid (Qwen3.5) models compute differently when sequences share a micro-batch, and one sequence per decode keeps answers bit-exact and independent of load
- a single-question decision takes one sequence; a multi-question decision also reserves a question lane, which is forked from the state for every question but the last, which runs on the state sequence itself
- the capacity ledger admits decisions in FIFO order once their sequences are free and their state plus longest question fits the unified KV cache
- user text is NFC-normalized and tokenized with special-token parsing off, matching the Hugging Face tokenizer kev was trained with (`fixtures/qwen3_5_tokenizer_reference.json` is the contract)
- it builds on `paddler_agent_runtime` and `paddler_agent_pointer_head`, and never depends on another pipeline crate
- `DecisionError` is its single error enum
