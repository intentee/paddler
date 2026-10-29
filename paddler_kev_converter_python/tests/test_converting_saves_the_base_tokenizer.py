from pathlib import Path

from kev.checkpoint import Checkpoint
from kev.model import SPECIAL, load_tokenizer
from transformers import AutoTokenizer

TOKENIZED_TEXT = "".join(SPECIAL) + " The café served crème brûlée at 9:30."


def test_converting_saves_the_base_tokenizer(
    converted_backbone_directory: Path, convertible_kev_run: str
) -> None:
    meta = Checkpoint(convertible_kev_run).meta

    assert (
        AutoTokenizer.from_pretrained(converted_backbone_directory)(TOKENIZED_TEXT).input_ids
        == load_tokenizer(meta.base, revision=meta.base_revision)(TOKENIZED_TEXT).input_ids
    )
