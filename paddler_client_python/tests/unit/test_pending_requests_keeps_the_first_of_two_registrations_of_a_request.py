import pytest

from paddler_client.error import ConnectionDroppedError, RequestIdInFlightError
from paddler_client.pending_requests import PendingRequests


async def test_pending_requests_keeps_the_first_of_two_registrations_of_a_request() -> (
    None
):
    pending_requests = PendingRequests()
    first_response_stream = pending_requests.register("request-1")

    with pytest.raises(RequestIdInFlightError) as request_id_in_flight:
        pending_requests.register("request-1")

    pending_requests.drain()

    with pytest.raises(ConnectionDroppedError) as connection_dropped:
        await anext(first_response_stream)

    assert request_id_in_flight.value.request_id == "request-1"
    assert connection_dropped.value.request_id == "request-1"
