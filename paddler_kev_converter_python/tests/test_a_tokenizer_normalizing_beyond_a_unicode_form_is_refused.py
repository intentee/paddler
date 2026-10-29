from pathlib import Path

import pytest
from tokenizers.normalizers import NFC, Lowercase, Sequence

from paddler_kev_converter.error import TokenizerNormalizerUnsupportedError
from paddler_kev_converter.tokenizer_input_normalization import (
    tokenizer_input_normalization,
)
from tests.tokenizer_normalizing_with import tokenizer_normalizing_with


def test_a_tokenizer_normalizing_beyond_a_unicode_form_is_refused(
    tmp_path: Path,
) -> None:
    with pytest.raises(TokenizerNormalizerUnsupportedError) as refusal:
        tokenizer_input_normalization(
            tokenizer_normalizing_with(Sequence([NFC(), Lowercase()]), tmp_path)
        )

    assert refusal.value.normalizer == "Sequence"
