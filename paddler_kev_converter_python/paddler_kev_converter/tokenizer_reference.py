import json
from pathlib import Path

from kev.checkpoint import Checkpoint
from kev.model import load_tokenizer

REFERENCE_TEXTS = [
    "Hello world",
    "  leading and trailing spaces  ",
    "several    spaces\tand\ttabs\nand\n\nnewlines",
    "1234567890 and 3.14159 and -42",
    "Don't stop, I'm sure we'll go; they've said: it's done!",
    "Za\u017c\u00f3\u0142\u0107 g\u0119\u015bl\u0105 ja\u017a\u0144",
    "Zaz\u0307o\u0301\u0142c\u0301 ge\u0328s\u0301la\u0328 jaz\u0301n\u0301",
    "e\u0301t\u00e9 and \u00e9t\u00e9",
    "\uff21\uff22\uff23 full width and \u2460 circled",
    "\u65e5\u672c\u8a9e\u306e\u30c6\u30ad\u30b9\u30c8\u3068\u4e2d\u6587\u6587\u672c",
    "\u0645\u0631\u062d\u0628\u0627 \u0628\u0627\u0644\u0639\u0627\u0644\u0645",
    "family \U0001f468\u200d\U0001f469\u200d\U0001f467\u200d\U0001f466",
    "flags \U0001f1f5\U0001f1f1\U0001f1fa\U0001f1f8",
    '{"key": [1, 2.5, true, null], "nested": {"a": "b"}}',
    "def add(left, right):\n    return left + right\n",
    "supercalifragilisticexpialidocious antidisestablishmentarianism",
]


def write_tokenizer_reference(checkpoint_run: str, path: Path) -> None:
    meta = Checkpoint(checkpoint_run).meta
    tokenizer = load_tokenizer(meta.base, revision=meta.base_revision)

    path.write_text(
        json.dumps(
            [
                {
                    "text": text,
                    "token_ids": tokenizer(text, add_special_tokens=False).input_ids,
                }
                for text in REFERENCE_TEXTS
            ],
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
