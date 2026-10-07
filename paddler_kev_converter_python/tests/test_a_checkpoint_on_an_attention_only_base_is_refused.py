from pathlib import Path

import pytest
from kev.checkpoint import read_meta, write_meta

from paddler_kev_converter.convertible_kev_checkpoint import ConvertibleKevCheckpoint
from paddler_kev_converter.error import BaseArchitectureUnsupportedError

ATTENTION_ONLY_BASE = "Qwen/Qwen3-0.6B-Base"
ATTENTION_ONLY_BASE_REVISION = "da87bfb608c14b7cf20ba1ce41287e8de496c0cd"


def test_a_checkpoint_on_an_attention_only_base_is_refused(
    kev_checkpoint_copy: Path,
) -> None:
    meta = read_meta(str(kev_checkpoint_copy))
    meta.base = ATTENTION_ONLY_BASE
    meta.base_revision = ATTENTION_ONLY_BASE_REVISION
    write_meta(str(kev_checkpoint_copy), meta)

    with pytest.raises(BaseArchitectureUnsupportedError) as refusal:
        ConvertibleKevCheckpoint.open(str(kev_checkpoint_copy))

    assert refusal.value.text_model_type == "qwen3"
