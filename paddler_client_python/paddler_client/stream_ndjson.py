from __future__ import annotations

import json
from typing import TYPE_CHECKING

from paddler_client.inference_message import (
    InferenceMessage,
    parse_inference_client_message,
)
from paddler_client.raise_for_streaming_error import raise_for_streaming_error

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    import httpx


async def stream_ndjson_inference_messages(
    response: httpx.Response,
) -> AsyncIterator[InferenceMessage]:
    await raise_for_streaming_error(response)

    async for raw_line in response.aiter_lines():
        stripped_line = raw_line.strip()

        if not stripped_line:
            continue

        data = json.loads(stripped_line)

        yield parse_inference_client_message(data)
