import torch
from kev.model import DecisionModel
from transformers import PreTrainedTokenizerBase

class Meta:
    base: str
    base_revision: str | None
    head: dict[str, torch.Tensor]
    option_isolation: bool
    temperature: float
    weights: str
    weights_dtype: str

class LoadOptions: ...

def read_meta(run: str) -> Meta: ...
def write_meta(run: str, meta: Meta) -> None: ...

class Checkpoint:
    meta: Meta
    path: str
    def __init__(self, run: str) -> None: ...
    def load(
        self, device: str, opts: LoadOptions = ...
    ) -> tuple[PreTrainedTokenizerBase, DecisionModel]: ...
