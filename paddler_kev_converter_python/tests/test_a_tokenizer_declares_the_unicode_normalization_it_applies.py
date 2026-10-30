from pathlib import Path

from tokenizers.normalizers import NFC, NFD, NFKC, NFKD, Normalizer

from paddler_kev_converter.input_normalization import InputNormalization
from paddler_kev_converter.tokenizer_input_normalization import (
    tokenizer_input_normalization,
)
from tests.tokenizer_normalizing_with import tokenizer_normalizing_with


def test_a_tokenizer_declares_the_unicode_normalization_it_applies(
    tmp_path: Path,
) -> None:
    normalizers: dict[InputNormalization, Normalizer | None] = {
        InputNormalization.NFC: NFC(),
        InputNormalization.NFD: NFD(),
        InputNormalization.NFKC: NFKC(),
        InputNormalization.NFKD: NFKD(),
        InputNormalization.NONE: None,
    }

    assert {
        input_normalization: tokenizer_input_normalization(
            tokenizer_normalizing_with(normalizer, tmp_path)
        )
        for input_normalization, normalizer in normalizers.items()
    } == {input_normalization: input_normalization for input_normalization in normalizers}
