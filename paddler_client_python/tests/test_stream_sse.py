import httpx
import pytest

from paddler_client.error import HttpError
from paddler_client.stream_sse import stream_sse


async def test_extracts_data_from_sse_lines() -> None:
    sse_content = 'data: {"agents": []}\ndata: {"agents": [{}]}\n'

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text=sse_content)

    transport = httpx.MockTransport(handler)

    async with httpx.AsyncClient(transport=transport) as http_client:
        async with http_client.stream("GET", "http://test/stream") as response:
            events: list[str] = []

            async for data in stream_sse(response):
                events.append(data)

    assert len(events) == 2
    assert events[0] == '{"agents": []}'
    assert events[1] == '{"agents": [{}]}'


async def test_skips_non_data_lines() -> None:
    sse_content = "event: update\nid: 1\ndata: payload\nretry: 1000\n"

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text=sse_content)

    transport = httpx.MockTransport(handler)

    async with httpx.AsyncClient(transport=transport) as http_client:
        async with http_client.stream("GET", "http://test/stream") as response:
            events: list[str] = []

            async for data in stream_sse(response):
                events.append(data)

    assert len(events) == 1
    assert events[0] == "payload"


async def test_raises_http_error_on_non_200() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(503, text="Service Unavailable")

    transport = httpx.MockTransport(handler)

    async with httpx.AsyncClient(transport=transport) as http_client:
        async with http_client.stream("GET", "http://test/stream") as response:
            with pytest.raises(HttpError) as exc_info:
                async for _data in stream_sse(response):
                    pass

    assert exc_info.value.status_code == 503


async def test_empty_stream_yields_nothing() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, text="")

    transport = httpx.MockTransport(handler)

    async with httpx.AsyncClient(transport=transport) as http_client:
        async with http_client.stream("GET", "http://test/stream") as response:
            events: list[str] = []

            async for data in stream_sse(response):
                events.append(data)

    assert len(events) == 0
