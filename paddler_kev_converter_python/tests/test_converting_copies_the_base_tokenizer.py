from pathlib import Path

from huggingface_hub import snapshot_download
from kev.checkpoint import Checkpoint

from paddler_kev_converter.convertible_kev_checkpoint import BASE_TOKENIZER_FILES


def test_converting_copies_the_base_tokenizer(
    converted_backbone_directory: Path, kev_checkpoint: Checkpoint
) -> None:
    base_snapshot = Path(
        snapshot_download(
            kev_checkpoint.meta.base,
            revision=kev_checkpoint.meta.base_revision,
            allow_patterns=BASE_TOKENIZER_FILES,
        )
    )

    assert all(
        (converted_backbone_directory / tokenizer_file).read_bytes()
        == (base_snapshot / tokenizer_file).read_bytes()
        for tokenizer_file in BASE_TOKENIZER_FILES
    )
