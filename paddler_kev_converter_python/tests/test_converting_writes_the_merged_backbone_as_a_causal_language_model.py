from pathlib import Path

import torch
from kev.checkpoint import Checkpoint, LoadOptions
from transformers import Qwen3_5ForCausalLM


def test_converting_writes_the_merged_backbone_as_a_causal_language_model(
    converted_backbone_directory: Path, kev_checkpoint: Checkpoint
) -> None:
    _, decision_model = kev_checkpoint.load("cpu", LoadOptions())
    merged_backbone = decision_model.lm.state_dict()
    written_backbone = Qwen3_5ForCausalLM.from_pretrained(
        converted_backbone_directory, dtype=torch.float32
    ).model.state_dict()

    assert written_backbone.keys() == merged_backbone.keys()
    assert all(
        torch.equal(written_backbone[name], merged_tensor)
        for name, merged_tensor in merged_backbone.items()
    )
