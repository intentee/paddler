import json
from pathlib import Path

from kev.checkpoint import Checkpoint
from kev.model import load_tokenizer

from paddler_kev_converter.command_line import main


def test_the_tokenizer_reference_pairs_texts_with_base_token_ids(
    kev_checkpoint: Checkpoint, tmp_path: Path
) -> None:
    base = kev_checkpoint.meta.base
    base_revision = kev_checkpoint.meta.base_revision
    assert base_revision is not None
    main(
        [
            "tokenizer-reference",
            "--base",
            base,
            "--base-revision",
            base_revision,
            "--output",
            str(tmp_path / "reference.json"),
        ]
    )
    tokenizer = load_tokenizer(base, revision=base_revision)
    reference = json.loads((tmp_path / "reference.json").read_text(encoding="utf-8"))

    assert reference[0] == {
        "text": "Hello world",
        "token_ids": tokenizer("Hello world", add_special_tokens=False).input_ids,
    }
