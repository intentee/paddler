from pathlib import Path

import torch
from kev.checkpoint import Checkpoint, LoadOptions
from transformers import Qwen3_5ForCausalLM


def test_converting_writes_the_decision_backbone_as_a_tied_causal_language_model(
    converted_backbone_directory: Path, convertible_kev_run: str
) -> None:
    _, decision_model = Checkpoint(convertible_kev_run).load("cpu", LoadOptions())
    decision_backbone = decision_model.lm.state_dict()
    written_causal_language_model = Qwen3_5ForCausalLM.from_pretrained(
        converted_backbone_directory, dtype="auto"
    )
    written_backbone = written_causal_language_model.model.state_dict()

    assert written_backbone.keys() == decision_backbone.keys()
    assert all(
        torch.equal(written_backbone[name], decision_tensor)
        for name, decision_tensor in decision_backbone.items()
    )
    assert torch.equal(
        written_causal_language_model.lm_head.weight,
        written_causal_language_model.model.embed_tokens.weight,
    )
