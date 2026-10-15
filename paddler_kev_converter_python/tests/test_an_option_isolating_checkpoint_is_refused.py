from pathlib import Path

import pytest
from kev.checkpoint import read_meta, write_meta

from paddler_kev_converter.convertible_kev_checkpoint import ConvertibleKevCheckpoint
from paddler_kev_converter.error import OptionIsolationUnsupportedError


def test_an_option_isolating_checkpoint_is_refused(kev_checkpoint_copy: Path) -> None:
    meta = read_meta(str(kev_checkpoint_copy))
    meta.option_isolation = True
    write_meta(str(kev_checkpoint_copy), meta)

    with pytest.raises(OptionIsolationUnsupportedError):
        ConvertibleKevCheckpoint.open(str(kev_checkpoint_copy))
