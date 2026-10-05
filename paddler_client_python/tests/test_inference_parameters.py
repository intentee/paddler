from paddler_client.inference_parameters import InferenceParameters
from paddler_client.pooling_type import PoolingType


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
