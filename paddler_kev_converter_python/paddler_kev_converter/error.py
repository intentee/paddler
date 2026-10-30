class KevConversionError(Exception):
    pass


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


class TokenizerNormalizerUnsupportedError(KevConversionError):
    def __init__(self, tokenizer: str, normalizer: str) -> None:
        self.tokenizer = tokenizer
        self.normalizer = normalizer
        super().__init__(
            f"{tokenizer} normalizes its input with {normalizer}, "
            "which a pointer head cannot declare"
        )


class TokenizerNormalizerUninspectableError(KevConversionError):
    def __init__(self, tokenizer: str, tokenizer_class: str) -> None:
        self.tokenizer = tokenizer
        self.tokenizer_class = tokenizer_class
        super().__init__(
            f"{tokenizer} is a {tokenizer_class}, "
            "which does not expose the normalizer it applies"
        )
