import pytest
from pydantic import ValidationError
from pydantic_core import PydanticSerializationError

from paddler_client.embedding_normalization_method import (
    EmbeddingNormalizationMethod,
)
from paddler_client.error import (
    InvalidEmbeddingNormalizationMethodError,
    InvalidRmsNormPayloadError,
    RmsNormEpsilonMissingError,
)
from tests.unit.validation_error_cause import validation_error_cause


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

    with pytest.raises(PydanticSerializationError) as rejection:
        method.model_dump(mode="json")

    assert isinstance(rejection.value.__cause__, RmsNormEpsilonMissingError)


def test_embedding_normalization_method_invalid_rms_norm_raises() -> None:
    with pytest.raises(ValidationError) as rejection:
        EmbeddingNormalizationMethod.model_validate({"RmsNorm": "not a dict"})

    assert isinstance(
        validation_error_cause(rejection.value), InvalidRmsNormPayloadError
    )


def test_embedding_normalization_method_invalid_data_raises() -> None:
    with pytest.raises(ValidationError) as rejection:
        EmbeddingNormalizationMethod.model_validate(42)

    assert isinstance(
        validation_error_cause(rejection.value),
        InvalidEmbeddingNormalizationMethodError,
    )


def test_embedding_normalization_method_unknown_variant_raises() -> None:
    with pytest.raises(ValidationError) as rejection:
        EmbeddingNormalizationMethod.model_validate({"Unknown": {}})

    assert isinstance(
        validation_error_cause(rejection.value),
        InvalidEmbeddingNormalizationMethodError,
    )


def test_rms_norm_serializes_to_the_wire_format() -> None:
    method = EmbeddingNormalizationMethod.rms_norm(epsilon=1e-6)
    dumped = method.model_dump(mode="json")

    assert dumped == {"RmsNorm": {"epsilon": 1e-6}}
