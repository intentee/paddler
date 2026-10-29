from tokenizers.normalizers import NFC, NFD, NFKC, NFKD
from transformers import PreTrainedTokenizerBase, PreTrainedTokenizerFast

from paddler_kev_converter.error import (
    TokenizerNormalizerUninspectableError,
    TokenizerNormalizerUnsupportedError,
)
from paddler_kev_converter.input_normalization import InputNormalization


def tokenizer_input_normalization(
    tokenizer: PreTrainedTokenizerBase,
) -> InputNormalization:
    if not isinstance(tokenizer, PreTrainedTokenizerFast):
        raise TokenizerNormalizerUninspectableError(
            tokenizer.name_or_path, type(tokenizer).__name__
        )

    match tokenizer.backend_tokenizer.normalizer:
        case None:
            return InputNormalization.NONE
        case NFC():
            return InputNormalization.NFC
        case NFD():
            return InputNormalization.NFD
        case NFKC():
            return InputNormalization.NFKC
        case NFKD():
            return InputNormalization.NFKD
        case unsupported_normalizer:
            raise TokenizerNormalizerUnsupportedError(
                tokenizer.name_or_path, type(unsupported_normalizer).__name__
            )
