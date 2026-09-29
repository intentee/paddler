import pytest
from pydantic import ValidationError

from paddler_client.error import (
    BatchSizeExceedsContextSizeError,
    PenaltiesWithoutWindowError,
    PenaltyWindowWithoutPenaltiesError,
)
from paddler_client.inference_parameters import InferenceParameters
from paddler_client.pooling_type import PoolingType
from tests.unit.validation_error_cause import validation_error_cause


def test_inference_parameters_defaults() -> None:
    params = InferenceParameters()

    assert params.temperature == 0.8
    assert params.context_size == 8192
    assert params.pooling_type == PoolingType.LAST
    assert params.enable_embeddings is False


def test_inference_parameters_serialization() -> None:
    params = InferenceParameters(temperature=0.5, top_k=40)
    dumped = params.model_dump(mode="json")

    assert dumped["temperature"] == 0.5
    assert dumped["top_k"] == 40


def test_inference_parameters_reject_a_batch_larger_than_the_context() -> None:
    with pytest.raises(ValidationError) as validation_error:
        InferenceParameters(n_batch=8192, context_size=4096)

    assert isinstance(
        validation_error_cause(validation_error.value),
        BatchSizeExceedsContextSizeError,
    )


def test_inference_parameters_reject_penalty_strengths_without_a_window() -> None:
    with pytest.raises(ValidationError) as validation_error:
        InferenceParameters(penalty_presence=0.8)

    assert isinstance(
        validation_error_cause(validation_error.value),
        PenaltiesWithoutWindowError,
    )


def test_inference_parameters_reject_a_window_without_penalty_strengths() -> None:
    with pytest.raises(ValidationError) as validation_error:
        InferenceParameters(penalty_last_n=64)

    assert isinstance(
        validation_error_cause(validation_error.value),
        PenaltyWindowWithoutPenaltiesError,
    )
