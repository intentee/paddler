import json

import httpx
import pytest

from paddler_client.error import HttpError
from paddler_client.inference_message import InferenceMessage, InferenceMessageKind
from paddler_client.stream_ndjson import stream_ndjson_inference_messages


def _make_token_line(request_id: str, token: str) -> str:
    return json.dumps(
        {
            "Response": {
                "request_id": request_id,
                "response": {"GeneratedToken": {"Token": token}},
            }
        }
    )


def _make_done_line(request_id: str) -> str:
    return json.dumps(
        {
            "Response": {
                "request_id": request_id,
                "response": {"GeneratedToken": "Done"},
            }
        }
    )


async def test_parses_multiple_ndjson_lines() -> None:
    ndjson_content = (
        _make_token_line("r1", "hello") + "\n" + _make_done_line("r1") + "\n"
    )

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text=ndjson_content)

    transport = httpx.MockTransport(handler)

    async with httpx.AsyncClient(transport=transport) as http_client:
        async with http_client.stream("GET", "http://test/stream") as response:
            messages: list[InferenceMessage] = []

            async for message in stream_ndjson_inference_messages(response):
                messages.append(message)

    assert len(messages) == 2
    assert messages[0].kind == InferenceMessageKind.TOKEN
    assert messages[0].token == "hello"
    assert messages[1].kind == InferenceMessageKind.DONE


async def test_skips_empty_lines() -> None:
    ndjson_content = "\n\n" + _make_done_line("r1") + "\n\n"

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text=ndjson_content)

    transport = httpx.MockTransport(handler)

    async with httpx.AsyncClient(transport=transport) as http_client:
        async with http_client.stream("GET", "http://test/stream") as response:
            messages: list[InferenceMessage] = []

            async for message in stream_ndjson_inference_messages(response):
                messages.append(message)

    assert len(messages) == 1
    assert messages[0].is_done


async def test_raises_http_error_on_non_200() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(500, text="Internal Server Error")

    transport = httpx.MockTransport(handler)

    async with httpx.AsyncClient(transport=transport) as http_client:
        async with http_client.stream("GET", "http://test/stream") as response:
            with pytest.raises(HttpError) as exc_info:
                async for _message in stream_ndjson_inference_messages(response):
                    pass

    assert exc_info.value.status_code == 500
