from dataclasses import dataclass
from pathlib import Path
from shutil import copyfile

import torch
from huggingface_hub import snapshot_download
from kev.checkpoint import Checkpoint, LoadOptions
from kev.model import SPECIAL
from peft import LoraConfig
from transformers import AutoConfig, PreTrainedModel, Qwen3_5ForCausalLM

from paddler_kev_converter.error import (
    BackboneDtypeUnsupportedError,
    BaseArchitectureUnsupportedError,
    FullWeightCheckpointUnsupportedError,
    OptionIsolationUnsupportedError,
    TrainedTokenEmbeddingsUnsupportedError,
)
from paddler_kev_converter.pointer_head import PointerHead

BASE_TOKENIZER_FILES = [
    "merges.txt",
    "tokenizer.json",
    "tokenizer_config.json",
    "vocab.json",
]
CONVERTIBLE_TEXT_MODEL_TYPE = "qwen3_5_text"
EXACTLY_MERGING_BACKBONE_DTYPE = "fp32"
LORA_WEIGHTS = "lora"
CAUSAL_LANGUAGE_MODEL_CLASS: type[PreTrainedModel] = Qwen3_5ForCausalLM


@dataclass(frozen=True)
class ConvertibleKevCheckpoint:
    checkpoint: Checkpoint

    @classmethod
    def open(cls, run: str) -> "ConvertibleKevCheckpoint":
        checkpoint = Checkpoint(run)
        meta = checkpoint.meta

        if meta.weights != LORA_WEIGHTS:
            raise FullWeightCheckpointUnsupportedError(checkpoint.path)

        if meta.weights_dtype != EXACTLY_MERGING_BACKBONE_DTYPE:
            raise BackboneDtypeUnsupportedError(checkpoint.path, meta.weights_dtype)

        if meta.option_isolation:
            raise OptionIsolationUnsupportedError(checkpoint.path)

        if LoraConfig.from_pretrained(checkpoint.path).trainable_token_indices:
            raise TrainedTokenEmbeddingsUnsupportedError(checkpoint.path)

        text_model_type = (
            AutoConfig.from_pretrained(meta.base, revision=meta.base_revision)
            .get_text_config()
            .model_type
        )

        if text_model_type != CONVERTIBLE_TEXT_MODEL_TYPE:
            raise BaseArchitectureUnsupportedError(meta.base, text_model_type)

        return cls(checkpoint=checkpoint)

    def pointer_head(self) -> PointerHead:
        meta = self.checkpoint.meta

        return PointerHead(
            delimiters=SPECIAL,
            key_bias=meta.head["k.bias"].numpy(),
            key_weight=meta.head["k.weight"].numpy(),
            query_bias=meta.head["q.bias"].numpy(),
            query_weight=meta.head["q.weight"].numpy(),
            temperature=meta.temperature,
        )

    def write_backbone(self, directory: Path) -> None:
        _, decision_model = self.checkpoint.load("cpu", LoadOptions())

        with torch.device("meta"):
            causal_language_model = CAUSAL_LANGUAGE_MODEL_CLASS(
                decision_model.lm.config
            )

        causal_language_model.model = decision_model.lm
        causal_language_model.tie_weights()
        causal_language_model.save_pretrained(directory)

        meta = self.checkpoint.meta
        base_snapshot = Path(
            snapshot_download(
                meta.base,
                revision=meta.base_revision,
                allow_patterns=BASE_TOKENIZER_FILES,
            )
        )

        for tokenizer_file in BASE_TOKENIZER_FILES:
            copyfile(base_snapshot / tokenizer_file, directory / tokenizer_file)
