from dataclasses import dataclass
from pathlib import Path

import torch
import transformers
from kev.checkpoint import Checkpoint, LoadOptions
from kev.model import SPECIAL
from peft import LoraConfig
from transformers import PreTrainedModel, PreTrainedTokenizerBase
from transformers.models.auto.modeling_auto import MODEL_FOR_CAUSAL_LM_MAPPING_NAMES

from paddler_kev_converter.error import (
    BackboneDtypeUnsupportedError,
    OptionIsolationUnsupportedError,
    TrainedTokenEmbeddingsUnsupportedError,
)
from paddler_kev_converter.pointer_head import PointerHead
from paddler_kev_converter.tokenizer_input_normalization import (
    tokenizer_input_normalization,
)

EXACTLY_MERGING_BACKBONE_DTYPE = "fp32"


def _require_exactly_mergeable_adapter(checkpoint: Checkpoint) -> None:
    if checkpoint.meta.weights_dtype != EXACTLY_MERGING_BACKBONE_DTYPE:
        raise BackboneDtypeUnsupportedError(
            checkpoint.path, checkpoint.meta.weights_dtype
        )

    if LoraConfig.from_pretrained(checkpoint.path).trainable_token_indices:
        raise TrainedTokenEmbeddingsUnsupportedError(checkpoint.path)


@dataclass(frozen=True)
class ConvertibleKevCheckpoint:
    base_tokenizer: PreTrainedTokenizerBase
    checkpoint: Checkpoint
    decision_backbone: PreTrainedModel

    @classmethod
    def open(cls, run: str) -> "ConvertibleKevCheckpoint":
        checkpoint = Checkpoint(run)

        if checkpoint.meta.option_isolation:
            raise OptionIsolationUnsupportedError(checkpoint.path)

        if not checkpoint.full:
            _require_exactly_mergeable_adapter(checkpoint)

        base_tokenizer, decision_model = checkpoint.load("cpu", LoadOptions())

        return cls(
            base_tokenizer=base_tokenizer,
            checkpoint=checkpoint,
            decision_backbone=decision_model.lm,
        )

    def pointer_head(self) -> PointerHead:
        meta = self.checkpoint.meta

        return PointerHead(
            delimiters=SPECIAL,
            input_normalization=tokenizer_input_normalization(self.base_tokenizer),
            key_bias=meta.head["k.bias"].numpy(),
            key_weight=meta.head["k.weight"].numpy(),
            query_bias=meta.head["q.bias"].numpy(),
            query_weight=meta.head["q.weight"].numpy(),
            temperature=meta.temperature,
        )

    def write_backbone(self, directory: Path) -> None:
        backbone_config = self.decision_backbone.config
        causal_language_model_class: type[PreTrainedModel] = getattr(
            transformers, MODEL_FOR_CAUSAL_LM_MAPPING_NAMES[backbone_config.model_type]
        )
        backbone_config.tie_word_embeddings = True

        with torch.device("meta"):
            causal_language_model = causal_language_model_class(backbone_config)

        setattr(
            causal_language_model,
            causal_language_model.base_model_prefix,
            self.decision_backbone,
        )
        causal_language_model.tie_weights()
        causal_language_model.save_pretrained(directory)
        self.base_tokenizer.save_pretrained(directory)
