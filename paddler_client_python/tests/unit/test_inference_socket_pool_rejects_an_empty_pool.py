import pytest

from paddler_client.error import InvalidSocketPoolSizeError
from paddler_client.inference_socket_pool import InferenceSocketPool


def test_inference_socket_pool_rejects_an_empty_pool() -> None:
    with pytest.raises(InvalidSocketPoolSizeError) as rejected:
        InferenceSocketPool(url="ws://127.0.0.1:8061", pool_size=0)

    assert rejected.value.pool_size == 0
