import json

from paddler_client.pending_requests import PendingRequests


def timeout_error_for(request_id: str) -> str:
    return json.dumps(
        {
            "Error": {
                "request_id": request_id,
                "error": {"code": 504, "description": "timed out"},
            }
        }
    )


async def test_pending_requests_ignores_a_message_for_an_unknown_request() -> None:
    pending_requests = PendingRequests()
    response_stream = pending_requests.register("request-1")

    pending_requests.dispatch(timeout_error_for("request-that-was-never-sent"))
    pending_requests.dispatch(timeout_error_for("request-1"))

    assert [message.request_id async for message in response_stream] == ["request-1"]
