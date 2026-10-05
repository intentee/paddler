import json

from paddler_client.inference_message import InferenceMessageKind
from paddler_client.pending_requests import PendingRequests


async def test_pending_requests_delivers_a_message_to_its_request() -> None:
    pending_requests = PendingRequests()
    response_stream = pending_requests.register("request-1")

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

    [message] = [message async for message in response_stream]

    assert message.kind == InferenceMessageKind.SERVER_ERROR
    assert message.error_code == 504
