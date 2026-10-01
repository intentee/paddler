import json

from paddler_client.pending_requests import PendingRequests


async def test_pending_requests_ignores_malformed_messages_it_cannot_attribute() -> (
    None
):
    pending_requests = PendingRequests()
    response_stream = pending_requests.register("request-1")

    pending_requests.dispatch("not json")
    pending_requests.dispatch("[1, 2, 3]")
    pending_requests.dispatch('{"Unknown": {}}')
    pending_requests.dispatch(
        json.dumps(
            {
                "Error": {
                    "request_id": "request-1",
                    "error": {"code": 504, "description": "timed out"},
                }
            }
        )
    )

    assert [message.error_code async for message in response_stream] == [504]
