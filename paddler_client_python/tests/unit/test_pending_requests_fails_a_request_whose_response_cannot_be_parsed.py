import json

import pytest

from paddler_client.error import JsonError
from paddler_client.pending_requests import PendingRequests


async def test_pending_requests_fails_a_request_whose_response_cannot_be_parsed() -> (
    None
):
    pending_requests = PendingRequests()
    response_stream = pending_requests.register("request-1")
    raw_message = json.dumps(
        {
            "Response": {
                "generated_by": None,
                "request_id": "request-1",
                "response": "NotAVariant",
            }
        }
    )

    pending_requests.dispatch(raw_message)

    with pytest.raises(JsonError) as json_error:
        await anext(response_stream)

    assert json_error.value.raw_data == raw_message
