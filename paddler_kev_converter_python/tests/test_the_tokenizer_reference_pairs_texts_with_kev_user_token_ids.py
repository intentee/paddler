import json
from pathlib import Path

from kev.checkpoint import Checkpoint
from kev.model import load_tokenizer, user_tokens

from paddler_kev_converter.command_line import main
from paddler_kev_converter.tokenizer_reference import REFERENCE_TEXTS


def test_the_tokenizer_reference_pairs_texts_with_kev_user_token_ids(
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

    assert json.loads((tmp_path / "reference.json").read_text(encoding="utf-8")) == [
        {"text": text, "token_ids": user_tokens(tokenizer, text)}
        for text in REFERENCE_TEXTS
    ]
