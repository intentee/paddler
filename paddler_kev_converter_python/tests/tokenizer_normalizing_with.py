from json import dumps
from pathlib import Path

from tokenizers import Tokenizer, models
from tokenizers.normalizers import Normalizer
from transformers import AutoTokenizer, PreTrainedTokenizerBase

TOKENIZER_CONFIG = {"tokenizer_class": "PreTrainedTokenizerFast"}


def tokenizer_normalizing_with(
    normalizer: Normalizer | None, directory: Path
) -> PreTrainedTokenizerBase:
    backend_tokenizer = Tokenizer(
        models.WordLevel(vocab={"[UNK]": 0}, unk_token="[UNK]")
    )
    backend_tokenizer.normalizer = normalizer
    backend_tokenizer.save(str(directory / "tokenizer.json"))
    (directory / "tokenizer_config.json").write_text(dumps(TOKENIZER_CONFIG))

    return AutoTokenizer.from_pretrained(directory)
