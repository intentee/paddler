from __future__ import annotations

from typing import TYPE_CHECKING

from paddler_client.raise_for_streaming_error import raise_for_streaming_error

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    import httpx


async def stream_sse(
    response: httpx.Response,
) -> AsyncIterator[str]:
    await raise_for_streaming_error(response)

    async for raw_line in response.aiter_lines():
        stripped_line = raw_line.rstrip("\r")

        if stripped_line.startswith("data: "):
            yield stripped_line[6:]
