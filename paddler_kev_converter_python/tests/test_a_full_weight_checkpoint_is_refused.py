from pathlib import Path

import pytest
from kev.checkpoint import read_meta, write_meta

from paddler_kev_converter.convertible_kev_checkpoint import ConvertibleKevCheckpoint
from paddler_kev_converter.error import FullWeightCheckpointUnsupportedError


def test_a_full_weight_checkpoint_is_refused(kev_checkpoint_copy: Path) -> None:
    meta = read_meta(str(kev_checkpoint_copy))
    meta.weights = "full"
    write_meta(str(kev_checkpoint_copy), meta)

    with pytest.raises(FullWeightCheckpointUnsupportedError):
        ConvertibleKevCheckpoint.open(str(kev_checkpoint_copy))
