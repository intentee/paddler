import pytest

from paddler_client.error import ConnectionDroppedError
from paddler_client.pending_requests import PendingRequests


async def test_pending_requests_drops_every_request_when_drained() -> None:
    pending_requests = PendingRequests()
    response_streams = [
        pending_requests.register(request_id)
        for request_id in ("request-1", "request-2")
    ]

    pending_requests.drain()

    for request_id, response_stream in zip(
        ("request-1", "request-2"), response_streams, strict=True
    ):
        with pytest.raises(ConnectionDroppedError) as connection_dropped:
            await anext(response_stream)

        assert connection_dropped.value.request_id == request_id
