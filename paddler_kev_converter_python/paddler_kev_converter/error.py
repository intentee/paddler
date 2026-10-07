class KevConversionError(Exception):
    pass


class FullWeightCheckpointUnsupportedError(KevConversionError):
    def __init__(self, checkpoint_path: str) -> None:
        self.checkpoint_path = checkpoint_path
        super().__init__(
            f"{checkpoint_path} holds full backbone weights; "
            "only LoRA checkpoints can be converted"
        )


class BackboneDtypeUnsupportedError(KevConversionError):
    def __init__(self, checkpoint_path: str, weights_dtype: str) -> None:
        self.checkpoint_path = checkpoint_path
        self.weights_dtype = weights_dtype
        super().__init__(
            f"{checkpoint_path} was trained on a {weights_dtype} backbone; "
            "only fp32 backbones merge exactly"
        )


class OptionIsolationUnsupportedError(KevConversionError):
    def __init__(self, checkpoint_path: str) -> None:
        self.checkpoint_path = checkpoint_path
        super().__init__(
            f"{checkpoint_path} isolates its options, "
            "which needs a block-causal mask Paddler does not build"
        )


class TrainedTokenEmbeddingsUnsupportedError(KevConversionError):
    def __init__(self, checkpoint_path: str) -> None:
        self.checkpoint_path = checkpoint_path
        super().__init__(
            f"{checkpoint_path} trains token embeddings, "
            "which cannot be merged into the backbone"
        )


class BaseArchitectureUnsupportedError(KevConversionError):
    def __init__(self, base: str, text_model_type: str) -> None:
        self.base = base
        self.text_model_type = text_model_type
        super().__init__(
            f"{base} is a {text_model_type} model; only qwen3_5_text bases convert"
        )
