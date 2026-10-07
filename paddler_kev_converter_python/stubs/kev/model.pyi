from typing import Any

import torch
from transformers import PreTrainedModel, PreTrainedTokenizerBase

SPECIAL: list[str]

def load_tokenizer(
    name: str, revision: str | None = None
) -> PreTrainedTokenizerBase: ...

class DecisionModel:
    lm: PreTrainedModel
    def encode(
        self, tok: PreTrainedTokenizerBase, rec: dict[str, Any], *, strict: bool
    ) -> dict[str, Any]: ...
    def probs(self, enc: dict[str, Any]) -> list[torch.Tensor]: ...
