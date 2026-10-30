import pytest
from transformers import ByT5Tokenizer

from paddler_kev_converter.error import TokenizerNormalizerUninspectableError
from paddler_kev_converter.tokenizer_input_normalization import (
    tokenizer_input_normalization,
)


def test_a_slow_tokenizer_cannot_declare_its_normalization() -> None:
    with pytest.raises(TokenizerNormalizerUninspectableError) as refusal:
        tokenizer_input_normalization(ByT5Tokenizer())

    assert refusal.value.tokenizer_class == "ByT5Tokenizer"
