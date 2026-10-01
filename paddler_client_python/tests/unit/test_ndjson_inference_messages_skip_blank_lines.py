import json
from collections.abc import AsyncGenerator

from paddler_client.ndjson_inference_messages import ndjson_inference_messages


async def blank_line_then_message_line() -> AsyncGenerator[str, None]:
    yield "  \r"
    yield json.dumps(
        {
            "Response": {
                "generated_by": None,
                "request_id": "req-1",
                "response": {"Embedding": "NoEmbeddingsProduced"},
            }
        }
    )


async def test_ndjson_inference_messages_skip_blank_lines() -> None:
    messages = [
        message
        async for message in ndjson_inference_messages(blank_line_then_message_line())
    ]

    assert [message.request_id for message in messages] == ["req-1"]
