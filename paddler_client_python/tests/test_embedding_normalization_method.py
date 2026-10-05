import pytest

from paddler_client.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)


def test_embedding_normalization_method_l2_serialization() -> None:
    method = EmbeddingNormalizationMethod.l2()
    serialized = method.model_dump(mode="json")

    assert isinstance(serialized, str)
    assert serialized == "L2"


def test_embedding_normalization_method_none_serialization() -> None:
    method = EmbeddingNormalizationMethod.none()
    serialized = method.model_dump(mode="json")

    assert isinstance(serialized, str)
    assert serialized == "None"


def test_embedding_normalization_method_rms_norm_serialization() -> None:
    method = EmbeddingNormalizationMethod.rms_norm(epsilon=0.001)

    assert method.model_dump(mode="json") == {"RmsNorm": {"epsilon": 0.001}}


def test_embedding_normalization_method_l2_deserialization() -> None:
    method = EmbeddingNormalizationMethod.model_validate("L2")

    assert method.variant == "L2"


def test_embedding_normalization_method_rms_norm_deserialization() -> None:
    method = EmbeddingNormalizationMethod.model_validate(
        {"RmsNorm": {"epsilon": 0.001}}
    )

    assert method.variant == "RmsNorm"
    assert method.epsilon == 0.001


def test_embedding_normalization_method_rms_norm_missing_epsilon_raises() -> None:
    method = EmbeddingNormalizationMethod(variant="RmsNorm", epsilon=None)

    with pytest.raises(ValueError, match="epsilon is required"):
        method.model_dump(mode="json")


def test_embedding_normalization_method_invalid_rms_norm_raises() -> None:
    with pytest.raises(ValueError, match="Invalid RmsNorm payload"):
        EmbeddingNormalizationMethod.model_validate({"RmsNorm": "not a dict"})


def test_embedding_normalization_method_invalid_data_raises() -> None:
    with pytest.raises(ValueError, match="Invalid EmbeddingNormalizationMethod"):
        EmbeddingNormalizationMethod.model_validate(42)


def test_embedding_normalization_method_rms_norm_roundtrip() -> None:
    method = EmbeddingNormalizationMethod.rms_norm(epsilon=1e-6)
    dumped = method.model_dump(mode="json")

    assert dumped == {"RmsNorm": {"epsilon": 1e-6}}
