from pathlib import Path

import pytest

from paddler_kev_converter.convertible_kev_checkpoint import ConvertibleKevCheckpoint
from paddler_kev_converter.error import TrainedTokenEmbeddingsUnsupportedError
from tests.conftest import rewrite_adapter_config


def test_a_checkpoint_training_token_embeddings_is_refused(
    kev_checkpoint_copy: Path,
) -> None:
    rewrite_adapter_config(
        kev_checkpoint_copy, trainable_token_indices={"embed_tokens": [248044]}
    )

    with pytest.raises(TrainedTokenEmbeddingsUnsupportedError):
        ConvertibleKevCheckpoint.open(str(kev_checkpoint_copy))
