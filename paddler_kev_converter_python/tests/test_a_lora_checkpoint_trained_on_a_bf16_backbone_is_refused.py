from pathlib import Path

import pytest
from kev.checkpoint import read_meta, write_meta

from paddler_kev_converter.convertible_kev_checkpoint import ConvertibleKevCheckpoint
from paddler_kev_converter.error import BackboneDtypeUnsupportedError


def test_a_lora_checkpoint_trained_on_a_bf16_backbone_is_refused(
    kev_checkpoint_copy: Path,
) -> None:
    meta = read_meta(str(kev_checkpoint_copy))
    meta.weights_dtype = "bf16"
    write_meta(str(kev_checkpoint_copy), meta)

    with pytest.raises(BackboneDtypeUnsupportedError) as refusal:
        ConvertibleKevCheckpoint.open(str(kev_checkpoint_copy))

    assert refusal.value.weights_dtype == "bf16"
